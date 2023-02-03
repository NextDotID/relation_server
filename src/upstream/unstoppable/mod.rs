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
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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
    #[serde(rename = "tokenId")]
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

async fn fetch_domains_by_account(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut next = String::from("");
    let mut next_targets: Vec<Target> = Vec::new();
    let client = make_client();
    loop {
        let uri: http::Uri;
        if next.len() == 0 {
            uri = format!(
                "{}/domains?owners={}",
                C.upstream.unstoppable_api.url, identity
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        } else {
            uri = format!(
                "{}/domains?owners={}&startingAfter={}",
                C.upstream.unstoppable_api.url, identity, next
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        }

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", "Bearer tokenId")
            .body(Body::empty())
            .expect("request builder");
        // .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

        let mut resp = client.request(req).await?;
        if !resp.status().is_success() {
            error!("Unstoppable fetch error, statusCode: {}", resp.status());
            break;
        }
        let result = match parse_body::<RecordsForOwnerResponse>(&mut resp).await {
            Ok(result) => result,
            Err(_) => {
                let err: BadResponse = parse_body(&mut resp).await?;
                error!(
                    "Unstoppable fetch error, Code: {}, Message: {}",
                    err.code, err.message
                );
                break;
            }
        };
        if result.data.len() == 0 {
            debug!("Unstoppable fetching no records");
            break;
        }

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
                platform: Platform::Unstoppable,
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
                let reverse_uri: http::Uri =
                    format!("{}/reverse/{}", C.upstream.unstoppable_api.url, identity)
                        .parse()
                        .map_err(|_err: InvalidUri| {
                            Error::ParamError(format!("Uri format Error {}", _err))
                        })?;
                let reverse_req = hyper::Request::builder()
                    .method(Method::GET)
                    .uri(reverse_uri)
                    .header("Authorization", "Bearer tokenId")
                    .body(Body::empty())
                    .expect("request builder");

                let mut reverse_resp = client.request(reverse_req).await?;
                if !reverse_resp.status().is_success() {
                    error!(
                        "Unstoppable reverse fetch error, statusCode: {}",
                        reverse_resp.status()
                    );
                    break;
                }
                let _reverse = match parse_body::<ReverseResponse>(&mut reverse_resp).await {
                    Ok(r) => r,
                    Err(_) => {
                        let err: BadResponse = parse_body(&mut resp).await?;
                        error!(
                            "Unstoppable reverse fetch error, Code: {}, Message: {}",
                            err.code, err.message
                        );
                        break;
                    }
                };

                let owner: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Ethereum,
                    identity: _reverse
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
                    name: _reverse.meta.domain,
                    fetcher: DataFetcher::RelationService,
                    updated_at: naive_now(),
                };
                resolve
                    .connect(&db, &unstoppable_record, &owner_record)
                    .await?;
            }
            next_targets.extend(vec![Target::Identity(
                Platform::Unstoppable,
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
    Ok(vec![])
}
