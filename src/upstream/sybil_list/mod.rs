extern crate futures;
#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::Proof;
use crate::tigergraph::upsert::create_identity_to_identity_proof_two_way_binding;
use crate::tigergraph::vertex::Identity;
use crate::tigergraph::EdgeList;
use crate::upstream::{DataSource, Fetcher, Platform, ProofLevel, TargetProcessedList};
use crate::util::make_http_client;
use crate::util::{make_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive};
use async_trait::async_trait;
use hyper::{client::HttpConnector, Client};
use hyper::{Body, Method};
use serde::Deserialize;
use serde_json::{Map, Value};
use tracing::info;

use uuid::Uuid;

use futures::future::join_all;

use super::{DataFetcher, Target};

#[derive(Deserialize, Debug)]
pub struct SybilListItem {
    pub twitter_name: String,
    pub eth_addr: String,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug)]
pub struct VerifiedItem {
    pub twitter: TwitterItem,
}

#[derive(Deserialize, Debug)]
pub struct TwitterItem {
    pub timestamp: i64,
    #[serde(rename = "tweetID")]
    pub tweet_id: String,
    pub handle: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct SybilList {}

async fn save_item(
    client: &Client<HttpConnector>,
    eth_wallet_address: String,
    value: Value,
) -> Option<(Platform, String)> {
    let item: VerifiedItem = serde_json::from_value(value).ok()?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: eth_wallet_address.to_lowercase(),
        uid: None,
        created_at: None,
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Twitter,
        identity: item.twitter.handle.to_lowercase(),
        uid: None,
        created_at: None,
        display_name: Some(item.twitter.handle.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    let create_ms_time: u32 = (item.twitter.timestamp % 1000).try_into().unwrap();
    let pf: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::SybilList,
        level: ProofLevel::VeryConfident,
        record_id: Some(item.twitter.tweet_id.clone()),
        created_at: timestamp_to_naive(item.twitter.timestamp / 1000, create_ms_time),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    let pb: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::SybilList,
        level: ProofLevel::VeryConfident,
        record_id: Some(item.twitter.tweet_id.clone()),
        created_at: timestamp_to_naive(item.twitter.timestamp / 1000, create_ms_time),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    create_identity_to_identity_proof_two_way_binding(&client, &from, &to, &pf, &pb)
        .await
        .ok()?;
    Some((Platform::Twitter, item.twitter.handle.clone()))
}

/// Trigger a refetch from github.
pub async fn prefetch() -> Result<(), Error> {
    let client = make_client();
    let uri: http::Uri = (C.upstream.sybil_service.url).parse().unwrap();

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("SybilList Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("SybilList fetch | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        return Err(Error::General(
            format!("SybilList Get error: {}", body.message),
            resp.status(),
        ));
    }

    // all records in sybil list
    let body: Map<String, Value> = parse_body(&mut resp).await?;

    // parse
    let cli = make_http_client();
    let futures: Vec<_> = body
        .into_iter()
        .map(|(eth_wallet_address, value)| save_item(&cli, eth_wallet_address, value))
        .collect();
    let _ = join_all(futures).await;
    Ok(())
}

#[async_trait]
impl Fetcher for SybilList {
    /// Only search sybil list in local database, no download process should occur.
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        let platform = target.platform()?;
        let identity = target.identity()?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(&client, &platform, &identity).await?;
        match found {
            Some(found) => {
                let nexts = found
                    .find_identity_by_source(&client, &DataSource::SybilList)
                    .await?;
                let next_target = nexts
                    .into_iter()
                    .filter(|f| f.platform == Platform::Ethereum || f.platform == Platform::Twitter)
                    .map(|next| Target::Identity(next.platform, next.identity.clone()))
                    .collect();
                Ok(next_target)
            }
            None => {
                info!(
                    "Sybil list: {} not found in local sybil list record",
                    target,
                );
                Ok(vec![])
            }
        }
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        // TODO: prefetch: move this logic to `data_process` module as a scheduled asynchronous fetch
        Ok((vec![], vec![]))
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Twitter])
    }
}
