#[cfg(test)]
mod tests;
use crate::{
    config::C,
    error::Error,
    graph::{
        create_identity_to_contract_record,
        edge::hold::Hold,
        new_db_connection,
        vertex::{contract::Chain, contract::ContractCategory, Contract, Identity},
    },
    upstream::{DataSource, Fetcher, Platform, Target, TargetProcessedList},
    util::{make_client, naive_now, parse_body},
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime};
use futures::future::join_all;
use http::uri::InvalidUri;
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

use super::DataFetcher;

#[derive(Deserialize, Debug)]
pub struct Rss3Response {
    pub version: String,
    #[serde(default)]
    pub date_updated: String,
    pub identifier: String,
    pub total: i64,
    pub list: Vec<Item>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub date_created: String,
    #[serde(default)]
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
    pub network: Rss3Chain,
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

/*
ethereum, ethereum_classic,
binance_smart_chain, polygon, zksync, xdai,
arweave, arbitrum, optimism, fantom, avalanche, crossbell
*/
#[derive(Deserialize, Debug)]
pub enum Rss3Chain {
    #[serde(rename = "ethereum")]
    Ethereum,

    #[serde(rename = "ethereum_classic")]
    EthereumClassic,

    #[serde(rename = "binance_smart_chain")]
    BinanceSmartChain,

    #[serde(rename = "polygon")]
    Polygon,

    #[serde(rename = "zksync")]
    Zksync,

    #[serde(rename = "xdai")]
    Xdai,

    #[serde(rename = "arweave")]
    Arweave,

    #[serde(rename = "arbitrum")]
    Arbitrum,

    #[serde(rename = "optimism")]
    Optimism,
}

impl From<Rss3Chain> for Chain {
    fn from(network: Rss3Chain) -> Self {
        match network {
            Rss3Chain::Ethereum => Chain::Ethereum,
            Rss3Chain::Polygon => Chain::Polygon,
            Rss3Chain::EthereumClassic => Chain::EthereumClassic,
            Rss3Chain::BinanceSmartChain => Chain::BNBSmartChain,
            Rss3Chain::Zksync => Chain::ZKSync,
            Rss3Chain::Xdai => Chain::Gnosis,
            Rss3Chain::Arweave => Chain::Arweave,
            Rss3Chain::Arbitrum => Chain::Arbitrum,
            Rss3Chain::Optimism => Chain::Optimism,
        }
    }
}

pub struct Rss3 {}

#[async_trait]
impl Fetcher for Rss3 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(platform, identity) => fetch_nfts_by_account(platform, identity).await,
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        // TODO: add NFT support
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}

async fn fetch_nfts_by_account(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}account:{}@{}/notes?tags=NFT",
        C.upstream.rss3_service.url, identity, platform
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    //.parse().map_err(|err| { Error::ParamError(...) })?;
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
            "rss3 Result Get Error".to_string(),
            resp.status(),
        ));
    }

    let futures: Vec<_> = body
        .list
        .into_iter()
        .filter(|p| p.metadata.to == identity.to_lowercase())
        .map(save_item)
        .collect();

    let next_targets: TargetProcessedList = join_all(futures)
        .await
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect();

    Ok(next_targets)
}

async fn save_item(p: Item) -> Result<TargetProcessedList, Error> {
    // Don't use ENS result returned from RSS3.
    if p.metadata.contract_type == *"ENS" {
        return Ok(vec![]);
    }
    let creataed_at = DateTime::parse_from_rfc3339(&p.date_created).unwrap();
    let created_at_naive = NaiveDateTime::from_timestamp(creataed_at.timestamp(), 0);

    let db = new_db_connection().await?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: p.metadata.to.to_lowercase(),
        created_at: Some(created_at_naive),
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let chain: Chain = p.metadata.network.into();
    let nft_category = ContractCategory::from_str(p.metadata.contract_type.as_str()).unwrap();

    let to: Contract = Contract {
        uuid: Uuid::new_v4(),
        category: nft_category,
        address: p.metadata.collection_address.clone().to_lowercase(),
        chain,
        symbol: Some(p.metadata.token_symbol.clone()),
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Rss3,
        transaction: Some(p.metadata.proof.clone()),
        id: p.metadata.token_id.clone(),
        created_at: Some(created_at_naive),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    create_identity_to_contract_record(&db, &from, &to, &hold).await?;

    Ok(vec![Target::NFT(
        chain,
        nft_category,
        p.metadata.collection_address.clone(),
        p.metadata.token_id.clone(),
    )])
}
