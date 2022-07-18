mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::{edge::Proof, edge::Hold};
use crate::graph::vertex::{Identity, Contract, contract::ContractCategory};
use crate::graph::{new_db_connection, Edge, Vertex};
use crate::upstream::{DataSource, Platform, Chain, Fetcher, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, timestamp_to_naive};
use futures::future::join_all;
use std::str::FromStr;
use uuid::Uuid;
use async_trait::async_trait;
use serde::Deserialize;
use super::Target;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub current: u32,
    pub next: u32,
}

#[derive(Deserialize, Debug)]
pub struct Record {
    pub id: String,
    pub sns_handle: String,
    pub sns_platform: String,
    pub web3_addr: String,
    pub web3_platform: String,
    pub source: String,
    pub ens: Option<String>,
    pub create_timestamp: String,
    pub modify_timestamp: String,
}

#[derive(Deserialize, Debug)]
pub struct Response {
    pub pagination: Pagination,
    pub records: Vec<Record>,
}
pub struct Aggregation {}

async fn save_item(p: Record) -> Option<Target> {
    let db = new_db_connection().await.ok()?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::from_str(p.sns_platform.as_str()).unwrap_or(Platform::Unknown),
        identity: p.sns_handle.clone(),
        created_at: None,
        display_name: p.sns_handle.clone(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let from_record = from.create_or_update(&db).await.ok()?;

    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::from_str(p.web3_platform.as_str()).unwrap(),
        identity: p.web3_addr.clone(),
        created_at: None,
        display_name: p.web3_addr.clone(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let to_record = to.create_or_update(&db).await.ok()?;

    let pf: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::from_str(p.source.as_str()).unwrap_or(DataSource::Unknown),
        record_id: Some(p.id.clone()),
        created_at: Some(timestamp_to_naive(p.create_timestamp.parse().unwrap())),
        updated_at: timestamp_to_naive(p.modify_timestamp.parse().unwrap()),
    };
    pf.connect(&db, &from_record, &to_record).await.ok()?;

    if p.ens.is_some() {
        let to_contract_identity: Contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::ENS,
            address: ContractCategory::ENS.default_contract_address().unwrap(),
            chain: Chain::Ethereum,
            symbol: None,
            updated_at: naive_now(),
        };
        let to_nft_record = to_contract_identity.create_or_update(&db).await.ok()?;

        let ownership: Hold = Hold {
            uuid: Uuid::new_v4(),
            transaction: None,
            id: p.ens.unwrap(),
            source: DataSource::from_str(p.source.as_str()).unwrap_or(DataSource::Unknown),
            created_at:None,
            updated_at: naive_now(),
        };
        ownership.connect(&db, &from_record, &to_nft_record).await.ok()?;
    }

    return Some(Target::Identity(
        Platform::from_str(p.web3_platform.as_str()).unwrap(),
        p.web3_addr.clone(),
    ));
}

#[async_trait]
impl Fetcher for Aggregation {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        let client = make_client();
        let mut page = 1;

        let mut res: TargetProcessedList = Vec::new();

        let platform = target.platform()?;
        let identity = target.identity()?;
        loop {
            let uri: http::Uri = match format!(
                "{}?platform={}&identity={}&page={}&size=100",
                C.upstream.aggregation_service.url, platform, identity, page
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
                break;
            }

            let body: Response = parse_body(&mut resp).await?;
            if body.records.len() == 0 {
                break;
            }

            // parse
            let futures: Vec<_> = body.records.into_iter().map(|p| save_item(p)).collect();
            let results = join_all(futures).await;
            let cons: TargetProcessedList = results.into_iter().filter_map(|i| i).collect();
            res.extend(cons);

            if body.pagination.current == body.pagination.next {
                break;
            }
            page = body.pagination.next;
        }

        Ok(res)
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Twitter])
    }
}
