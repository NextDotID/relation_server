mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::create_identity_to_identity_hold_record;
use crate::graph::edge::Edge;
use crate::graph::edge::Resolve;
use crate::graph::edge::{hold::Hold, resolve::DomainNameSystem};
use crate::graph::vertex::Vertex;
use crate::graph::{new_db_connection, vertex::Identity};
use crate::upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, timestamp_to_naive};
use async_trait::async_trait;
use http::uri::InvalidUri;
use hyper::{Body, Method};
use serde::Deserialize;
use tracing::{debug, error};
use uuid::Uuid;

use super::types::target;

#[derive(Deserialize, Debug, Clone)]
pub struct BadResponse {
    pub code: String,
    pub message: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DomainResponse {
    pub meta: Meta,
    pub records: Records,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReverseResponse {
    pub meta: Meta,
    pub records: Records,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RecordsForOwnerResponse {
    pub data: Vec<Item>,
    pub meta: MetaList,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MetaList {
    #[serde(rename = "perPage")]
    pub per_page: i64,
    #[serde(rename = "nextStartingAfter")]
    pub next: String,
    #[serde(rename = "sortBy")]
    pub sort_by: String,
    #[serde(rename = "sortDirection")]
    pub sort_direction: String,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: String,
    pub attributes: Attributes,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Attributes {
    pub meta: Meta,
    pub records: Records,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Meta {
    pub domain: String,
    #[serde(rename = "tokenId")]
    pub token_id: Option<String>,
    pub namehash: Option<String>,
    pub blockchain: Option<String>,
    #[serde(rename = "networkId")]
    pub network_id: i64,
    pub owner: Option<String>,
    pub resolver: Option<String>,
    pub registry: Option<String>,
    pub reverse: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Records {
    #[serde(rename = "crypto.BTC.address")]
    pub btc_address: Option<String>,
    #[serde(rename = "crypto.ETH.address")]
    pub eth_address: Option<String>,
}

pub struct Unstoppable {}
#[async_trait]
impl Fetcher for Unstoppable {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(platform, identity) => {
                fetch_connections_by_platform_identity(platform, identity).await
            }
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::UnstoppableDomains, Platform::Ethereum])
    }
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Ethereum => fetch_domains_by_account(platform, identity).await,
        Platform::UnstoppableDomains => fetch_account_by_domain(platform, identity).await,
        _ => Ok(vec![]),
    }
}

async fn fetch_domain(owners: &str, page: &str) -> Result<RecordsForOwnerResponse, Error> {
    let client = make_client();
    let uri: http::Uri = if page.is_empty() {
        format!(
            "{}/domains?owners={}",
            C.upstream.unstoppable_api.url, owners
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    } else {
        format!(
            "{}/domains?owners={}&startingAfter={}",
            C.upstream.unstoppable_api.url, owners, page
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    };

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut resp = client.request(req).await?;
    if !resp.status().is_success() {
        let err_message = format!("Unstoppable fetch error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }
    // Parse response body
    let result: RecordsForOwnerResponse = match parse_body(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut resp).await?;
            let err_message = format!(
                "Unstoppable fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    Ok(result)
}

async fn fetch_reverse(owner: &str) -> Result<ReverseResponse, Error> {
    let client = make_client();
    let reverse_uri: http::Uri = format!("{}/reverse/{}", C.upstream.unstoppable_api.url, owner)
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let reverse_req = hyper::Request::builder()
        .method(Method::GET)
        .uri(reverse_uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .expect("request builder");

    let mut reverse_resp = client.request(reverse_req).await?;
    if !reverse_resp.status().is_success() {
        let err_message = format!(
            "Unstoppable reverse fetch error, statusCode: {}",
            reverse_resp.status()
        );
        error!(err_message);
        return Err(Error::General(err_message, reverse_resp.status()));
    };
    let result = match parse_body::<ReverseResponse>(&mut reverse_resp).await {
        Ok(r) => r,
        Err(_) => {
            let err: BadResponse = parse_body(&mut reverse_resp).await?;
            let err_message = format!(
                "Unstoppable reverse fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            error!(err_message);

            return Err(Error::General(err_message, reverse_resp.status()));
        }
    };

    Ok(result)
}

async fn fetch_domains_by_account(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut next = String::from("");
    let mut next_targets: Vec<Target> = Vec::new();
    let client = make_client();
    loop {
        let result = fetch_domain(identity, &next).await?;
        let db = new_db_connection().await?;

        let eth_identity: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: identity.to_string().to_lowercase().clone(),
            created_at: None,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
        };
        let eth_record = eth_identity.create_or_update(&db).await?;
        for item in result.data.into_iter() {
            let unstoppable_identity: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::UnstoppableDomains,
                identity: item.attributes.meta.domain.clone(),
                created_at: None,
                display_name: Some(item.attributes.meta.domain.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Unstoppable,
                transaction: None,
                id: item.attributes.meta.token_id.unwrap_or("".to_string()),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };
            let unstoppable_record = unstoppable_identity.create_or_update(&db).await?;
            hold.connect(&db, &eth_record, &unstoppable_record).await?;
            // reverse = true
            if item.attributes.meta.reverse {
                let reverse_result = fetch_reverse(identity).await?;

                let owner: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Ethereum,
                    identity: reverse_result
                        .meta
                        .owner
                        .unwrap_or(identity.to_string().clone())
                        .to_lowercase(),
                    created_at: None,
                    display_name: None,
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: None,
                    updated_at: naive_now(),
                };
                let owner_record = owner.create_or_update(&db).await?;

                let resolve: Resolve = Resolve {
                    uuid: Uuid::new_v4(),
                    source: DataSource::Unstoppable,
                    system: DomainNameSystem::Unstoppable,
                    name: reverse_result.meta.domain,
                    fetcher: DataFetcher::RelationService,
                    updated_at: naive_now(),
                };
                resolve
                    .connect(&db, &unstoppable_record, &owner_record)
                    .await?;
            }
            next_targets.extend(vec![Target::Identity(
                Platform::UnstoppableDomains,
                item.attributes.meta.domain.clone(),
            )]);
        }

        if result.meta.has_more {
            next = result.meta.next;
        } else {
            break;
        }
    }
    Ok(next_targets)
}

async fn fetch_account_by_domain(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();
    let uri: http::Uri = format!("{}/domains/{}", C.upstream.unstoppable_api.url, identity)
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .expect("request builder");
    let mut resp = client.request(req).await?;
    if !resp.status().is_success() {
        error!("Unstoppable fetch error, statusCode: {}", resp.status());
    }
    let result = match parse_body::<DomainResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut resp).await?;
            error!(
                "Unstoppable fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            return Ok(vec![]);
        }
    };

    if result.meta.owner.is_none() {
        return Ok(vec![]);
    }

    let db = new_db_connection().await?;
    let eth_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: result.meta.owner.clone().unwrap().to_lowercase(),
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let unstoppable_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::UnstoppableDomains,
        identity: result.meta.domain.clone(),
        created_at: None,
        display_name: Some(result.meta.domain.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Unstoppable,
        transaction: None,
        id: result.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    let eth_record = eth_identity.create_or_update(&db).await?;
    let unstoppable_record = unstoppable_identity.create_or_update(&db).await?;
    hold.connect(&db, &eth_record, &unstoppable_record).await?;
    let target = Target::Identity(
        Platform::Ethereum,
        result.meta.owner.clone().unwrap().to_lowercase(),
    );
    // reverse = true
    if result.meta.reverse {
        let reverse_uri: http::Uri = format!(
            "{}/reverse/{}",
            C.upstream.unstoppable_api.url,
            result.meta.owner.clone().unwrap().to_lowercase()
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

        let reverse_req = hyper::Request::builder()
            .method(Method::GET)
            .uri(reverse_uri)
            .header(
                "Authorization",
                format!("Bearer {}", C.upstream.unstoppable_api.token),
            )
            .body(Body::empty())
            .expect("request builder");

        let mut reverse_resp = client.request(reverse_req).await?;
        if !reverse_resp.status().is_success() {
            error!(
                "Unstoppable reverse fetch error, statusCode: {}",
                reverse_resp.status()
            );
            return Ok(vec![target]);
        }
        let _reverse = match parse_body::<ReverseResponse>(&mut reverse_resp).await {
            Ok(r) => r,
            Err(_) => {
                let err: BadResponse = parse_body(&mut resp).await?;
                error!(
                    "Unstoppable reverse fetch error, Code: {}, Message: {}",
                    err.code, err.message
                );
                return Ok(vec![target]);
            }
        };
        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Unstoppable,
            system: DomainNameSystem::Unstoppable,
            name: _reverse.meta.domain,
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        resolve
            .connect(&db, &unstoppable_record, &eth_record)
            .await?;
    }

    Ok(vec![target])
}
