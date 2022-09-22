#[cfg(test)]
mod tests;
use crate::config::C;
use crate::error::Error;
use crate::graph::{edge::Proof, new_db_connection, vertex::Identity};
use crate::graph::{Edge, Vertex, create_identity_to_identity_record};
use crate::upstream::{DataSource, Fetcher, Platform, Target, TargetProcessedList, DataFetcher};
use crate::util::{naive_now, timestamp_to_naive};
use async_trait::async_trait;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;
use reqwest;

pub struct DotBit {}

#[async_trait]
impl Fetcher for DotBit {
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
        target.in_platform_supported(vec![Platform::Dotbit])
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct RequestParams {
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DotbitRequest {
    pub jsonrpc: String,
    pub id: i32,
    pub method: String,
    pub params: Vec<RequestParams>,
}

/**
 "out_point":{
    "tx_hash":"0x422d26080d682c067b76e0f27a426ab6e100a05f0c9dbc39e34c74300ec457c3",
    "index":0
},
"account_info":{
    "account":"test0920.bit",
    "account_alias":"test0920.bit",
    "account_id_hex":"0x2029f95fe87c3be5ad1a7107122c32ed56760ce1",
    "next_account_id_hex":"0x202a1c02ed5531fde99616fe5e844f781dc51df2",
    "create_at_unix":1663655157,
    "expired_at_unix":1695191157,
    "status":0,
    "das_lock_arg_hex":"0x054271b15dca69f8c1c942c64028dbd3b84c5d03b0054271b15dca69f8c1c942c64028dbd3b84c5d03b0",
    "owner_algorithm_id":5,
    "owner_key":"0x4271b15dca69f8c1c942c64028dbd3b84c5d03b0",
    "manager_algorithm_id":5,
    "manager_key":"0x4271b15dca69f8c1c942c64028dbd3b84c5d03b0"
}

 */
#[derive(Debug, Serialize, Deserialize)]
pub struct OutPoint {
    pub tx_hash: String,
    pub index: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub account: String,
    pub account_alias: String,
    pub account_id_hex: String,
    pub create_at_unix: i64,
    pub expired_at_unix: i64,
    pub owner_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub out_point: Option<OutPoint>,
    pub account_info: Option<AccountInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DotbitResult {
    pub errno: Option<i32>,
    pub errmsg: String,
    pub data: Option<Data>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DotbitResponse {
    pub id: Option<i32>,
    pub jsonrpc: String,
    pub result: DotbitResult,
}


async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let request_acc = RequestParams {
        account: identity.to_string(),
    };
    let params = DotbitRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "das_accountInfo".to_string(),
        params: vec![request_acc],
    };

    let client = reqwest::Client::new();
    let resp: DotbitResponse = client
        .post("https://indexer-v1.did.id")
        .json(&params)
        .send()
        .await?
        .json()
        .await?;
    if resp.result.errno.unwrap() != 0 {
        info!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }
    let info = resp.result.data.unwrap();
    let account_info = info.account_info.unwrap();
    let out_point = info.out_point.unwrap();
    // add to db 


    let db = new_db_connection().await?;
    let created_at_naive = timestamp_to_naive(account_info.create_at_unix, 0);
    
    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Dotbit,
        identity: identity.to_string(),
        created_at: Some(created_at_naive),
        display_name: Some(identity.to_string()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let to_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: account_info.owner_key.clone(),
        created_at: Some(created_at_naive),
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let pf: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        record_id: Some(out_point.tx_hash),
        created_at: Some(created_at_naive),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    
    create_identity_to_identity_record(&db, &from, &to_identity, &pf).await?;
    return Ok(vec![Target::Identity(Platform::Ethereum, account_info.owner_key)]);

}
