extern crate futures;
#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::edge::ProofRecord;
use crate::graph::{edge::Proof, new_db_connection, vertex::Identity};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, timestamp_to_naive};
use aragog::query::{Comparison, Filter, QueryResult};
use aragog::{DatabaseConnection, DatabaseRecord, EdgeRecord, Record};
use async_trait::async_trait;
use http::StatusCode;
use tracing::{debug, info};
use serde::Deserialize;

use serde_json::{Map, Value};

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
    db: &DatabaseConnection,
    eth_wallet_address: String,
    value: Value,
) -> Option<(Platform, String)> {
    let item: VerifiedItem = serde_json::from_value(value).ok()?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: eth_wallet_address.to_lowercase(),
        created_at: None,
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let from_record = from.create_or_update(db).await.ok()?;

    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Twitter,
        identity: item.twitter.handle.to_lowercase(),
        created_at: None,
        display_name: Some(item.twitter.handle.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let to_record = to.create_or_update(db).await.ok()?;

    let create_ms_time: u32 = (item.twitter.timestamp % 1000).try_into().unwrap();
    let proof: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::SybilList,
        record_id: Some(item.twitter.tweet_id),
        created_at: Some(timestamp_to_naive(
            item.twitter.timestamp / 1000,
            create_ms_time,
        )), // millisecond
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    proof.connect(db, &from_record, &to_record).await.ok()?;
    Some((Platform::Twitter, item.twitter.handle.clone()))
}

/// Trigger a refetch from github.
pub async fn prefetch() -> Result<(), Error> {
    let client = make_client();
    let uri: http::Uri = (C.upstream.sybil_service.url).parse().unwrap();

    let mut resp = client.get(uri).await?;

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
    let db = new_db_connection().await?;
    let futures: Vec<_> = body
        .into_iter()
        .map(|(eth_wallet_address, value)| save_item(&db, eth_wallet_address, value))
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
        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(&db, &platform, &identity).await?;
        if found.is_none() {
            info!(
                "Sybil list: {} not found in local sybil list record",
                target,
            );
            return Ok(vec![]);
        }

        match platform {
            // FIXME: stupid.
            Platform::Ethereum => {
                let filter =
                    Filter::new(Comparison::field("_from").equals_str(found.unwrap().id()))
                        .and(Comparison::field("source").equals_str(DataSource::SybilList));
                let result: QueryResult<EdgeRecord<Proof>> = EdgeRecord::<Proof>::query()
                    .filter(filter)
                    .call(&db)
                    .await?;

                if result.len() == 0 {
                    debug!("No sybil list record found for {}", identity);
                    Ok(vec![])
                } else {
                    let found: ProofRecord = result.first().unwrap().clone().into();
                    let next_target: DatabaseRecord<Identity> = found.record.to_record(&db).await?;

                    Ok(vec![Target::Identity(
                        next_target.platform,
                        next_target.identity.clone(),
                    )])
                }
            }
            Platform::Twitter => {
                let filter = Filter::new(Comparison::field("_to").equals_str(found.unwrap().id()))
                    .and(Comparison::field("source").equals_str(DataSource::SybilList));
                let result: QueryResult<EdgeRecord<Proof>> = EdgeRecord::<Proof>::query()
                    .filter(filter)
                    .call(&db)
                    .await?;

                if result.len() == 0 {
                    debug!("No sybil list record found for {}", identity);
                    Ok(vec![])
                } else {
                    let found: ProofRecord = result.first().unwrap().clone().into();
                    let next_target: DatabaseRecord<Identity> =
                        found.record.from_record(&db).await?;

                    Ok(vec![Target::Identity(
                        next_target.platform,
                        next_target.identity.clone(),
                    )])
                }
            }
            _ => Err(Error::General(
                format!("Platform not supported in sybil_list fetcher: {}", platform),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Twitter])
    }
}
