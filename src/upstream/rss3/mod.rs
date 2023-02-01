#[cfg(test)]
mod tests;
use crate::{
    config::C,
    error::Error,
    graph::{
        create_identity_to_contract_record,
        edge::{hold::Hold, proof::Proof},
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
use tracing::{error, info};
use uuid::Uuid;

use super::{
    types::{platform, target},
    DataFetcher,
};

#[derive(Deserialize, Debug, Clone)]
pub struct Rss3Response {
    pub total: i64,
    pub cursor: Option<String>,
    pub result: Vec<ResultItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResultItem {
    pub timestamp: String,
    #[serde(default)]
    pub hash: String,
    pub owner: String,
    pub address_from: String,
    #[serde(default)]
    pub address_to: String,
    pub network: String,
    pub platform: Option<String>,
    pub tag: String,
    #[serde(rename = "type")]
    pub tag_type: String,
    pub success: bool,
    pub actions: Vec<ActionItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ActionItem {
    pub tag: String,
    #[serde(rename = "type")]
    pub tag_type: String,
    #[serde(default)]
    pub hash: String,
    pub index: i64,
    pub address_from: String,
    #[serde(default)]
    pub address_to: String,
    pub metadata: MetaData,
    #[serde(default)]
    pub related_urls: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MetaData {
    pub id: Option<String>,
    pub name: Option<String>,
    pub image: Option<String>,
    pub value: Option<String>,
    pub symbol: Option<String>,
    pub standard: Option<String>,
    pub contract_address: Option<String>,
    pub handle: Option<String>,
}

const PAGE_LIMIT: i64 = 500;
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
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}

async fn fetch_nfts_by_account(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut cursor = String::from("");
    let client = make_client();
    let mut next_targets = Vec::new();

    loop {
        let uri: http::Uri;
        if cursor.len() == 0 {
            uri = format!(
                "{}/{}?tag=collectible&include_poap=true&refresh=true",
                C.upstream.rss3_service.url, identity
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        } else {
            uri = format!(
                "{}/{}?tag=collectible&include_poap=true&refresh=true&cursor={}",
                C.upstream.rss3_service.url, identity, cursor
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        }

        let mut resp = client.get(uri).await?;
        if !resp.status().is_success() {
            error!("Rss3 fetch error, statusCode: {}", resp.status());
            break;
        }
        let body: Rss3Response = parse_body(&mut resp).await?;
        if body.total == 0 {
            info!("Rss3 Response result is empty");
            break;
        }

        let futures: Vec<_> = body
            .result
            .into_iter()
            .filter(|p| p.owner == identity.to_lowercase())
            .map(save_item)
            .collect();

        let targets: TargetProcessedList = join_all(futures)
            .await
            .into_iter()
            .flat_map(|result| result.unwrap_or_default())
            .collect();

        next_targets.extend(targets);
        if body.cursor.is_none() || body.total < PAGE_LIMIT {
            break;
        } else {
            cursor = body.cursor.unwrap();
        }
    }

    Ok(next_targets)
}

async fn save_item(p: ResultItem) -> Result<TargetProcessedList, Error> {
    let creataed_at = DateTime::parse_from_rfc3339(&p.timestamp).unwrap();
    let created_at_naive = NaiveDateTime::from_timestamp(creataed_at.timestamp(), 0);
    let db = new_db_connection().await?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: p.owner.to_lowercase(),
        created_at: Some(created_at_naive),
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    if p.actions.len() == 0 {
        return Ok(vec![]);
    }

    let found = p
        .actions
        .iter()
        // collectible (transfer, mint, burn) share the same UMS, but approve/revoke not.
        .filter(|a| {
            (a.tag_type == "transfer" && p.tag_type == "transfer")
                || (a.tag_type == "mint" && p.tag_type == "mint")
                || (a.tag_type == "burn" && p.tag_type == "burn")
        })
        .find(|a| (p.tag == "collectible" && a.tag == "collectible"));
    if found.is_none() {
        return Ok(vec![]);
    }
    let real_action = found.unwrap();

    if real_action.metadata.symbol.is_none()
        || real_action.metadata.symbol.as_ref().unwrap() == &String::from("ENS")
    {
        return Ok(vec![]);
    }

    let mut nft_category =
        ContractCategory::from_str(real_action.metadata.standard.as_ref().unwrap().as_str())
            .unwrap_or_default();

    if real_action.tag_type == "poap".to_string() {
        nft_category = ContractCategory::POAP;
    }

    let chain = Chain::from_str(p.network.as_str()).unwrap_or_default();
    if chain == Chain::Unknown {
        error!("Rss3 Fetch data | Unknown Chain, original data: {:?}", p);
        return Ok(vec![]);
    }
    let contract_addr = real_action
        .metadata
        .contract_address
        .as_ref()
        .unwrap()
        .to_lowercase();
    let nft_id = real_action.metadata.id.as_ref().unwrap();

    let to: Contract = Contract {
        uuid: Uuid::new_v4(),
        category: nft_category,
        address: contract_addr.clone(),
        chain,
        symbol: Some(real_action.metadata.symbol.as_ref().unwrap().clone()),
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Rss3,
        transaction: Some(p.hash),
        id: nft_id.clone(),
        created_at: Some(created_at_naive),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    create_identity_to_contract_record(&db, &from, &to, &hold).await?;

    Ok(vec![Target::NFT(
        chain,
        nft_category,
        contract_addr.clone(),
        nft_id.clone(),
    )])
}
