#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{Hold, HyperEdge, Wrapper, HOLD_CONTRACT, HYPER_EDGE};
use crate::tigergraph::vertex::{Contract, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    Chain, ContractCategory, DataSource, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{make_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive};
use async_trait::async_trait;
use http::uri::InvalidUri;
use hyper::{Body, Method};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, error};
use uuid::Uuid;

use super::DataFetcher;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rss3ErrorResponse {
    pub error: String,
    pub error_code: String,
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetAccountActivitiesResponse {
    pub data: Option<Vec<Activitity>>,
    pub meta: Option<Cursor>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cursor {
    pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activitity {
    pub id: String,
    pub owner: String,
    pub network: String,
    pub index: i32,
    #[serde(rename = "from")]
    pub address_from: String,
    #[serde(rename = "to")]
    pub address_to: String,
    pub tag: String,
    #[serde(rename = "type")]
    pub tag_type: String,
    pub success: bool,
    pub direction: String, // in/out/self
    pub timestamp: i64,
    pub actions: Vec<ActionItem>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionItem {
    pub tag: String,
    #[serde(rename = "type")]
    pub tag_type: String,
    #[serde(rename = "from")]
    pub address_from: String,
    #[serde(rename = "to")]
    pub address_to: String,
    pub metadata: MetaData,
    #[serde(default)]
    pub related_urls: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaData {
    pub address: Option<String>,
    pub id: Option<String>,
    pub value: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub standard: Option<String>,
    pub decimals: Option<i32>,
    pub image: Option<String>,            // deprecated
    pub contract_address: Option<String>, // deprecated
    pub handle: Option<String>,           // deprecated
}

const PAGE_LIMIT: usize = 100;
pub struct Rss3 {}

#[async_trait]
impl Fetcher for Rss3 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(_platform, _identity) => todo!(),
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        match target.platform()? {
            Platform::Ethereum => batch_fetch_nfts(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}

async fn batch_fetch_nfts(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let client = make_client();
    let address = target.identity()?.to_lowercase();
    let mut current_cursor = String::from("");

    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    loop {
        let uri: http::Uri;
        if current_cursor.len() == 0 {
            uri = format!(
                "{}/decentralized/{}?tag=collectible&network=base,ethereum,optimism,polygon",
                C.upstream.rss3_service.url, address
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        } else {
            uri = format!(
                "{}/decentralized/{}?tag=collectible&network=base,ethereum,optimism,polygon&cursor={}",
                C.upstream.rss3_service.url, address, current_cursor
            )
            .parse()
            .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        }

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .map_err(|_err| Error::ParamError(format!("Rss3 Build Request Error {}", _err)))?;

        let mut resp = request_with_timeout(&client, req, None)
            .await
            .map_err(|err| {
                Error::ManualHttpClientError(format!(
                    "Rss3 fetch fetch | error: {:?}",
                    err.to_string()
                ))
            })?;

        let body = match parse_body::<GetAccountActivitiesResponse>(&mut resp).await {
            Ok(result) => result,
            Err(_) => match parse_body::<Rss3ErrorResponse>(&mut resp).await {
                Ok(rss3_info) => {
                    let err_message = format!(
                        "Rss3 Response error: {:?}, {:?}, {:?}",
                        rss3_info.error_code, rss3_info.error, rss3_info.detail
                    );
                    error!(err_message);
                    return Err(Error::ManualHttpClientError(err_message));
                }
                Err(err) => {
                    let err_message = format!("Rss3 Response error parse_body error: {:?}", err);
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
            },
        };

        if body.data.is_none() {
            debug!("Rss3 Response result is empty");
            break;
        }

        if let Some(meta) = body.meta {
            match meta.cursor {
                Some(cursor) => current_cursor = cursor,
                None => current_cursor = String::from(""),
            }
        } else {
            current_cursor = String::from("")
        }

        let result: Vec<Activitity> = body
            .data
            .clone()
            .map_or(vec![], |data: Vec<Activitity>| data)
            .into_iter()
            .filter(|p| p.owner.to_lowercase() == address)
            .filter(|p| {
                p.network == "base"
                    || p.network == "ethereum"
                    || p.network == "optimism"
                    || p.network == "polygon"
            })
            .collect();

        for p in result.into_iter() {
            if p.actions.len() == 0 {
                continue;
            }

            let found = p
                .actions
                .iter()
                // collectible (transfer, mint, burn) share the same UMS, but approve/revoke not.
                // we need to record is the `hold` relation, so burn is excluded
                .filter(|a| {
                    (a.tag_type == "transfer" && p.tag_type == "transfer")
                        || (a.tag_type == "mint" && p.tag_type == "mint")
                })
                .find(|a| (p.tag == "collectible" && a.tag == "collectible"));

            if found.is_none() {
                continue;
            }

            let real_action = found.unwrap();
            if real_action.metadata.symbol.is_none()
                || real_action.metadata.symbol.as_ref().unwrap() == &String::from("ENS")
            {
                continue;
            }

            let mut nft_category = ContractCategory::Unknown;
            let standard = real_action.metadata.standard.clone();
            if let Some(standard) = standard {
                if standard == "ERC-721".to_string() {
                    nft_category = ContractCategory::ERC721;
                } else if standard == "ERC-1155".to_string() {
                    nft_category = ContractCategory::ERC1155;
                }
            }
            if real_action.tag_type == "poap".to_string() {
                nft_category = ContractCategory::POAP;
            }
            let created_at_naive = timestamp_to_naive(p.timestamp, 0);

            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: p.owner.to_lowercase(),
                uid: None,
                created_at: created_at_naive,
                // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let chain = Chain::from_str(p.network.as_str()).unwrap_or_default();
            if chain == Chain::Unknown {
                error!("Rss3 Fetch data | Unknown Chain, original data: {:?}", p);
                continue;
            }

            let contract_addr = real_action
                .metadata
                .address
                .as_ref()
                .unwrap()
                .to_lowercase();

            let nft_id = real_action.metadata.id.as_ref().unwrap();
            let tx = real_action
                .related_urls
                .first()
                .cloned()
                .unwrap_or("".to_string());

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
                transaction: Some(tx),
                id: nft_id.clone(),
                created_at: created_at_naive,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: None,
            };

            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &from, HYPER_EDGE),
            ));
            let hdc = hold.wrapper(&from, &to, HOLD_CONTRACT);
            edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
        }

        if body.data.clone().unwrap().len() < PAGE_LIMIT {
            break;
        }
    }

    Ok((vec![], edges))
}
