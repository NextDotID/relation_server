mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::edge::Edge;
use crate::graph::edge::Resolve;
use crate::graph::edge::{hold::Hold, resolve::DomainNameSystem};
use crate::graph::vertex::IdentityRecord;
use crate::graph::vertex::Vertex;
use crate::graph::{new_db_connection, vertex::Identity};
use crate::upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, request_with_timeout};
use aragog::DatabaseConnection;
use async_trait::async_trait;
use futures::future::join_all;
use http::uri::InvalidUri;
use hyper::{Body, Method};
use serde::Deserialize;
use tracing::{error, warn};
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

const UNKNOWN_OWNER: &str = "0x0000000000000000000000000000000000000000";

pub struct UnstoppableDomains {}
#[async_trait]
impl Fetcher for UnstoppableDomains {
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

    let mut body = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    // Parse response body
    let result: RecordsForOwnerResponse = match parse_body(&mut body).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut body).await?;
            let err_message = format!(
                "UnstoppableDomains fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            error!(err_message);
            return Err(Error::General(
                err_message,
                lambda_http::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };
    Ok(result)
}

/// Temporarily do not use `reverse` query
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
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut reverse_resp = request_with_timeout(&client, reverse_req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch reverse record: {:?}",
                err.to_string()
            ))
        })?;

    let result = match parse_body::<ReverseResponse>(&mut reverse_resp).await {
        Ok(r) => r,
        Err(_) => {
            let err: BadResponse = parse_body(&mut reverse_resp).await?;
            let err_message = format!(
                "UnstoppableDomains reverse fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            error!(err_message);

            return Err(Error::General(err_message, reverse_resp.status()));
        }
    };

    Ok(result)
}

async fn save_domain(
    db: &DatabaseConnection,
    eth_record: &IdentityRecord,
    item: Item,
) -> Result<TargetProcessedList, Error> {
    let identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::UnstoppableDomains,
        identity: item.id.clone(),
        created_at: None,
        display_name: Some(item.attributes.meta.domain.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        transaction: None,
        id: item.attributes.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    let domain_record = identity.create_or_update(&db).await?;
    hold.connect(db, eth_record, &domain_record).await?;

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        system: DomainNameSystem::UnstoppableDomains,
        name: item.id.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // 'regular' resolution involves mapping from a name to an address.
    resolve.connect(db, &domain_record, eth_record).await?;

    if item.attributes.meta.reverse {
        // reverse = true
        // 'reverse' resolution maps from an address back to a name.
        resolve.connect(db, eth_record, &domain_record).await?;
        return Ok(vec![Target::Identity(
            Platform::UnstoppableDomains,
            item.attributes.meta.domain.clone(),
        )]);
    }
    Ok(vec![])
}

async fn fetch_domains_by_account(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut cnt: u32 = 0;
    let mut next = String::from("");
    let mut next_targets: Vec<Target> = Vec::new();
    while cnt < u32::MAX {
        let result = fetch_domain(identity, &next).await?;
        let db = new_db_connection().await?;
        cnt += result.data.len() as u32;

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
        let futures: Vec<_> = result
            .data
            .into_iter()
            .map(|item| save_domain(&db, &eth_record, item))
            .collect();

        let targets: TargetProcessedList = join_all(futures)
            .await
            .into_iter()
            .flat_map(|result| result.unwrap_or_default())
            .collect();

        next_targets.extend(targets);
        if result.meta.has_more {
            next = result.meta.next;
        } else {
            break;
        }
    }
    Ok(next_targets)
}

async fn fetch_owner(domains: &str) -> Result<DomainResponse, Error> {
    let client = make_client();
    let uri: http::Uri = format!("{}/domains/{}", C.upstream.unstoppable_api.url, domains)
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
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    let result = match parse_body::<DomainResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut resp).await?;
            let err_message = format!(
                "UnstoppableDomains fetch | errCode: {}, errMessage: {}",
                err.code, err.message
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    Ok(result)
}

async fn fetch_account_by_domain(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let db = new_db_connection().await?;
    let result = fetch_owner(identity).await?;
    if result.meta.owner.is_none() {
        return Ok(vec![]);
    }

    if result.meta.owner.clone().unwrap().to_lowercase() == UNKNOWN_OWNER {
        warn!("UnstoppableDomains owner is zero address");
        return Err(Error::NoResult);
    }

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

    let identity: Identity = Identity {
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
        source: DataSource::UnstoppableDomains,
        transaction: None,
        id: result.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    let eth_record = eth_identity.create_or_update(&db).await?;
    let domain_record = identity.create_or_update(&db).await?;
    hold.connect(&db, &eth_record, &domain_record).await?;

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        system: DomainNameSystem::UnstoppableDomains,
        name: result.meta.domain.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // 'regular' resolution involves mapping from a name to an address.
    resolve.connect(&db, &domain_record, &eth_record).await?;

    if result.meta.reverse {
        // reverse = true
        // 'reverse' resolution maps from an address back to a name.
        resolve.connect(&db, &eth_record, &domain_record).await?;
    }

    Ok(vec![Target::Identity(
        Platform::Ethereum,
        result.meta.owner.clone().unwrap().to_lowercase(),
    )])
}
