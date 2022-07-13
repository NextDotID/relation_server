mod tests;

use std::str::FromStr;

use crate::config::C;
use crate::graph::vertex::{nft::Chain, nft::NFTCategory, Identity, NFT};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body};
use crate::{
    error::Error,
    graph::{edge::Own, new_db_connection},
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime};
use futures::future::join_all;
use futures::TryFutureExt;
use serde::Deserialize;
use uuid::Uuid;

use super::Target;

#[derive(Deserialize, Debug)]
pub struct Rss3Response {
    pub version: String,
    pub date_updated: String,
    pub identifier: String,
    pub total: i64,
    pub list: Vec<Item>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub date_created: String,
    pub date_updated: String,
    pub related_urls: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub title: String,
    pub source: String,
    pub metadata: MetaData,
}

#[derive(Deserialize, Debug)]
pub struct MetaData {
    #[serde(default)]
    pub collection_address: String,
    #[serde(default)]
    pub collection_name: String,
    #[serde(default)]
    pub contract_type: String,
    pub from: String,
    #[serde(default)]
    pub log_index: String,
    pub network: String,
    pub proof: String,
    pub to: String,
    #[serde(default)]
    pub token_id: String,
    #[serde(default)]
    pub token_address: String,
    pub token_standard: String,
    pub token_symbol: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Rss3 {}

async fn save_item(p: Item) -> Result<TargetProcessedList, Error> {
    let creataed_at = DateTime::parse_from_rfc3339(&p.date_created).unwrap();
    let created_at_naive = NaiveDateTime::from_timestamp(creataed_at.timestamp(), 0);

    let db = new_db_connection().await?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: p.metadata.to.to_lowercase(),
        created_at: Some(created_at_naive),
        display_name: p.metadata.to.to_lowercase(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let from_record = from.create_or_update(&db).await?;

    let chain = Chain::from_str(p.metadata.network.as_str()).unwrap();
    let nft_category = NFTCategory::from_str(p.metadata.contract_type.as_str()).unwrap();
    let to: NFT = NFT {
        uuid: Uuid::new_v4(),
        category: nft_category.clone(),
        contract: p.metadata.collection_address.to_lowercase(),
        id: p.metadata.token_id.clone(),
        chain: chain.clone(),
        symbol: Some(p.metadata.token_symbol.clone()),
        updated_at: naive_now(),
    };

    let to_record = to.create_or_update(&db).await?;

    let ownership: Own = Own {
        uuid: Uuid::new_v4(),
        source: DataSource::Rss3,
        transaction: Some(p.metadata.proof.clone()),
    };

    ownership.connect(&db, &from_record, &to_record).await?;

    Ok(vec![Target::NFT(
        chain,
        nft_category,
        p.metadata.token_id.clone(),
    )])
}

#[async_trait]
impl Fetcher for Rss3 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        let platform = target.platform()?;
        let identity = target.identity()?;

        let client = make_client();
        let uri: http::Uri = match format!(
            "{}account:{}@{}/notes?tags=NFT",
            C.upstream.rss3_service.url, identity, platform
        )
        .parse()
        {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::ParamError(format!(
                    "Uri format Error: {}",
                    err.to_string()
                )))
            }
        };

        let mut resp = client.get(uri).await?;
        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Rss3 Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let body: Rss3Response = parse_body(&mut resp).await?;
        if body.total == 0 {
            return Err(Error::General(
                format!("rss3 Result Get Error"),
                resp.status(),
            ));
        }

        let futures: Vec<_> = body
            .list
            .into_iter()
            .filter(|p| p.metadata.to == identity.to_lowercase())
            .map(|p| save_item(p))
            .collect();
        let next_targets: TargetProcessedList = join_all(futures)
            .await
            .into_iter()
            .flat_map(|result| result.unwrap_or(vec![]))
            .collect();

        Ok(next_targets)
    }

    fn can_fetch(target: &Target) -> bool {
        // TODO: add NFT support
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}
