#[cfg(test)]
mod tests;
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
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

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
        target.in_platform_supported(vec![Platform::Dotbit, Platform::Ethereum])
    }
}

/// API docs https://github.com/dotbitHQ/das-account-indexer/blob/main/API.md
#[derive(Debug, Serialize, Deserialize)]
pub struct AccInfoRequestParams {
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccInfoRequest {
    pub jsonrpc: String,
    pub id: i32,
    pub method: String,
    pub params: Vec<AccInfoRequestParams>,
}

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
pub struct AccountInfoData {
    pub out_point: Option<OutPoint>,
    pub account_info: Option<AccountInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfoResult {
    pub errno: Option<i32>,
    pub errmsg: String,
    pub data: Option<AccountInfoData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfoResponse {
    pub id: Option<i32>,
    pub jsonrpc: String,
    pub result: AccountInfoResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestKeyInfo {
    pub coin_type: String,
    pub chain_id: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestTypeKeyInfoParams {
    #[serde(rename = "type")]
    pub req_type: String,
    pub key_info: RequestKeyInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReverseRecordRequest {
    pub jsonrpc: String,
    pub id: i32,
    pub method: String,
    pub params: Vec<RequestTypeKeyInfoParams>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountItem {
    pub account: String,
    pub account_alias: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReverseResult {
    pub errno: Option<i32>,
    pub errmsg: String,
    pub data: Option<AccountItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReverseResponse {
    pub id: Option<i32>,
    pub jsonrpc: String,
    pub result: ReverseResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountListData {
    pub account_list: Vec<AccountItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountListResult {
    pub errno: Option<i32>,
    pub errmsg: String,
    pub data: Option<AccountListData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountListResponse {
    pub id: Option<i32>,
    pub jsonrpc: String,
    pub result: AccountListResult,
}

const UNKNOWN_OWNER: &str = "0x0000000000000000000000000000000000000000";

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Dotbit => fetch_connections_by_account_info(platform, identity).await,
        Platform::Ethereum => fetch_hold_acc_and_reverse_record_by_addrs(platform, identity).await,
        _ => Ok(vec![]),
    }
}

async fn fetch_connections_by_account_info(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let request_acc = AccInfoRequestParams {
        account: identity.to_string(),
    };
    let params = AccInfoRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "das_accountInfo".to_string(),
        params: vec![request_acc],
    };
    let json_params = serde_json::to_vec(&params)?;

    let client = make_client();
    let req = Request::builder()
        .method(Method::POST)
        .uri(C.upstream.dotbit_service.url.clone())
        .body(Body::from(json_params))
        .expect("request builder");

    let mut result = client.request(req).await?;

    let resp: AccountInfoResponse = parse_body(&mut result).await?;
    if resp.result.errno.unwrap() != 0 {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }
    let info = resp.result.data.unwrap();
    let account_info = info.account_info.unwrap();
    let out_point = info.out_point.unwrap();

    // tricky way to remove the unexpected case...
    // will be removed after confirmied with .bit team how to define its a .bit NFT on Ethereum
    // https://talk.did.id/t/convert-your-bit-to-nft-on-ethereum-now/481
    if account_info.owner_key == UNKNOWN_OWNER {
        warn!(".bit profile owner is zero address");
        return Err(Error::NoResult);
    }

    // add to db
    let db = new_db_connection().await?;
    let created_at_naive = timestamp_to_naive(account_info.create_at_unix, 0);

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: account_info.owner_key.to_lowercase().clone(),
        created_at: Some(created_at_naive),
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let to: Identity = Identity {
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

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        transaction: Some(out_point.tx_hash),
        id: out_point.index.to_string(),
        created_at: Some(created_at_naive),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    create_identity_to_identity_hold_record(&db, &from, &to, &hold).await?;

    return Ok(vec![Target::Identity(
        Platform::Ethereum,
        account_info.owner_key,
    )]);
}

async fn fetch_hold_acc_and_reverse_record_by_addrs(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    fetch_account_list_by_addrs(_platform, identity).await?;
    // das_reverseRecord
    let request_params = get_req_params_by_platform(_platform, identity);
    let params = ReverseRecordRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "das_reverseRecord".to_string(),
        params: vec![request_params],
    };
    let json_params = serde_json::to_vec(&params)?;

    let client = make_client();
    let req = Request::builder()
        .method(Method::POST)
        .uri(C.upstream.dotbit_service.url.clone())
        .body(Body::from(json_params))
        .expect("request builder");

    let mut result = client.request(req).await?;

    let resp: ReverseResponse = parse_body(&mut result).await?;
    if resp.result.errno.unwrap() != 0 {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }
    if resp.result.data.is_none() || resp.result.data.as_ref().unwrap().account.len() == 0 {
        return Err(Error::NoResult);
    }

    let result_data = resp.result.data.unwrap();
    let db = new_db_connection().await?;
    let eth_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: identity.to_string().to_lowercase(),
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let dotbit_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Dotbit,
        identity: result_data.account.clone(),
        created_at: None,
        display_name: Some(result_data.account.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        transaction: None,
        id: "".to_string(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    let eth_record = eth_identity.create_or_update(&db).await?;
    let dotbit_record = dotbit_identity.create_or_update(&db).await?;
    hold.connect(&db, &eth_record, &dotbit_record).await?;

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        system: DomainNameSystem::DotBit,
        name: result_data.account.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };
    resolve.connect(&db, &dotbit_record, &eth_record).await?;

    return Ok(vec![Target::Identity(
        Platform::Dotbit,
        result_data.account.clone(),
    )]);
}

async fn fetch_account_list_by_addrs(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    // das_accountList
    let request_params = get_req_params_by_platform(_platform, identity);
    let params = ReverseRecordRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "das_accountList".to_string(),
        params: vec![request_params],
    };
    let json_params = serde_json::to_vec(&params)?;

    let client = make_client();
    let req = Request::builder()
        .method(Method::POST)
        .uri(C.upstream.dotbit_service.url.clone())
        .body(Body::from(json_params))
        .expect("request builder");

    let mut result = client.request(req).await?;

    let resp: AccountListResponse = parse_body(&mut result).await?;
    if resp.result.errno.unwrap() != 0 || resp.result.data.is_none() {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }

    let db = new_db_connection().await?;
    let from: Identity = Identity {
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
    let from_record = from.create_or_update(&db).await?;

    for i in resp.result.data.unwrap().account_list.into_iter() {
        let to: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Dotbit,
            identity: i.account.to_string(),
            created_at: None,
            display_name: Some(i.account.to_string()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Dotbit,
            transaction: None,
            id: "".to_string(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };

        let to_record = to.create_or_update(&db).await?;
        hold.connect(&db, &from_record, &to_record).await?;
    }

    Ok(vec![])
}

fn get_req_params_by_platform(_platform: &Platform, identity: &str) -> RequestTypeKeyInfoParams {
    // will support other platform later
    let req_key_info: RequestKeyInfo = RequestKeyInfo {
        coin_type: "60".to_string(),
        chain_id: "1".to_string(),
        key: identity.to_string().to_lowercase(),
    };
    return RequestTypeKeyInfoParams {
        req_type: "blockchain".to_string(),
        key_info: req_key_info,
    };
}
