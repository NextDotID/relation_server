#[cfg(test)]
mod tests;
use crate::config::C;
use crate::error::Error;
use crate::tigergraph::create_identity_domain_resolve_record;
use crate::tigergraph::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::create_identity_to_identity_hold_record;
use crate::tigergraph::edge::{Hold, Resolve};
use crate::tigergraph::vertex::Identity;
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{
    make_client, make_http_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive,
};
use async_trait::async_trait;
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};
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
        target.in_platform_supported(vec![
            Platform::Dotbit,
            Platform::Ethereum,
            Platform::CKB,
            Platform::Tron,
            Platform::Polygon,
            Platform::BNBSmartChain,
        ])
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
    pub owner_algorithm_id: i64,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfoData {
    pub out_point: Option<OutPoint>,
    pub account_info: Option<AccountInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfoResult {
    pub errno: Option<i32>,
    pub errmsg: Option<String>,
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
    pub errmsg: Option<String>,
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
    pub errmsg: Option<String>,
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
        Platform::CKB => fetch_hold_acc_and_reverse_record_by_addrs(platform, identity).await,
        Platform::Tron => fetch_hold_acc_and_reverse_record_by_addrs(platform, identity).await,
        Platform::Polygon => fetch_hold_acc_and_reverse_record_by_addrs(platform, identity).await,
        Platform::BNBSmartChain => {
            fetch_hold_acc_and_reverse_record_by_addrs(platform, identity).await
        }
        _ => Ok(vec![]),
    }
}

async fn fetch_connections_by_account_info(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let addr_identity = fetch_and_save_account_info(_platform, identity).await?;
    if addr_identity.is_none() {
        return Ok(vec![]);
    }
    let account_platform = addr_identity.clone().unwrap().platform;
    let account_identity = addr_identity.clone().unwrap().identity.clone();

    // fetch addr's reverse record
    let reverse_identity = fetch_reverse_record(&account_platform, &account_identity).await?;
    if reverse_identity.is_none() {
        return Ok(vec![Target::Identity(account_platform, account_identity)]);
    }
    let reverse_platform = reverse_identity.clone().unwrap().platform;
    let reverse_identity = reverse_identity.clone().unwrap().identity;

    return Ok(vec![
        Target::Identity(account_platform, account_identity),
        Target::Identity(reverse_platform, reverse_identity),
    ]);
}

async fn fetch_hold_acc_and_reverse_record_by_addrs(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    // fetch addr's hold dotbit records
    fetch_account_list_by_addrs(_platform, identity).await?;

    // fetch addr's reverse record
    let reverse_identity = fetch_reverse_record(_platform, identity).await?;
    if reverse_identity.is_none() {
        return Err(Error::NoResult);
    }

    return Ok(vec![Target::Identity(
        Platform::Dotbit,
        reverse_identity.unwrap().identity,
    )]);
}

async fn fetch_and_save_account_info(
    _platform: &Platform,
    identity: &str,
) -> Result<Option<Identity>, Error> {
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
        .map_err(|_err| Error::ParamError(format!("Dotbit Build Request Error {}", _err)))?;

    let mut result = request_with_timeout(&client, req, Some(std::time::Duration::from_secs(30)))
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Dotbit fetch | das_accountInfo error: {:?}",
                err.to_string()
            ))
        })?;

    let resp: AccountInfoResponse = parse_body(&mut result).await?;
    if resp.result.errno.map_or(false, |e| e != 0) {
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

    let account_addr = account_info.owner_key.to_lowercase();
    let account_platform: Platform = account_info.owner_algorithm_id.into();
    if account_platform == Platform::Unknown {
        warn!(
            ".bit profile owner_algorithm_id(value={}) map to platform is Unknown",
            account_info.owner_algorithm_id
        );
        return Ok(None);
    }

    let cli = make_http_client(); // connect server
    let created_at_naive = timestamp_to_naive(account_info.create_at_unix, 0);

    let addr_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: account_platform,
        identity: account_addr.clone(),
        uid: None,
        created_at: created_at_naive,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let dotbit_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Dotbit,
        identity: identity.to_string(),
        uid: None,
        created_at: created_at_naive,
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
        created_at: created_at_naive,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        system: DomainNameSystem::DotBit,
        name: identity.to_string(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // hold record
    create_identity_to_identity_hold_record(&cli, &addr_identity, &dotbit_identity, &hold).await?;
    // 'regular' resolution involves mapping from a name to an address.
    create_identity_domain_resolve_record(&cli, &dotbit_identity, &addr_identity, &resolve).await?;

    Ok(Some(addr_identity))
}

async fn fetch_reverse_record(
    _platform: &Platform,
    identity: &str,
) -> Result<Option<Identity>, Error> {
    let coin_type: CoinType = _platform.clone().into();
    if coin_type == CoinType::Unknown {
        return Ok(None);
    }
    // fetch addr's reverse record: das_reverseRecord
    let request_params = get_req_params(&coin_type, identity);
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
        .map_err(|_err| Error::ParamError(format!("Dotbit Build Request Error {}", _err)))?;

    let mut result = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Dotbit fetch | das_reverseRecord error: {:?}",
                err.to_string()
            ))
        })?;
    let resp: ReverseResponse = parse_body(&mut result).await?;
    if resp.result.errno.map_or(false, |e| e != 0) {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }
    if resp.result.data.is_none() || resp.result.data.as_ref().unwrap().account.len() == 0 {
        warn!("das_reverseRecord result is empty, resp {:?}", resp);
        return Ok(None);
    }

    let result_data = resp.result.data.unwrap();
    let cli = make_http_client(); // connect server

    let reverse_dotbit: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Dotbit,
        identity: result_data.account.clone(),
        uid: None,
        created_at: None,
        display_name: Some(result_data.account.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let reverse: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Dotbit,
        system: DomainNameSystem::DotBit,
        name: result_data.account.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    let addr_identity =
        fetch_and_save_account_info(&Platform::Dotbit, &result_data.account).await?;
    if addr_identity.is_none() {
        return Ok(None);
    }

    // das_reverseRecord: 'reverse' resolution maps from an address back to a name.
    create_identity_domain_reverse_resolve_record(
        &cli,
        &addr_identity.unwrap(),
        &reverse_dotbit,
        &reverse,
    )
    .await?;

    return Ok(Some(reverse_dotbit));
}

async fn fetch_account_list_by_addrs(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    // das_accountList
    let coin_type: CoinType = _platform.clone().into();
    if coin_type == CoinType::Unknown {
        return Ok(vec![]);
    }
    let request_params = get_req_params(&coin_type, identity);
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
        .map_err(|_err| Error::ParamError(format!("Dotbit Build Request Error {}", _err)))?;

    let mut result = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Dotbit fetch | das_accountList error: {:?}",
                err.to_string()
            ))
        })?;

    let resp: AccountListResponse = parse_body(&mut result).await?;
    if resp.result.errno.map_or(false, |e| e != 0) {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }
    if resp.result.data.is_none() {
        warn!("fail to fetch the result from .bit, resp {:?}", resp);
        return Err(Error::NoResult);
    }

    let cli = make_http_client(); // connect server
    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: identity.to_string().to_lowercase().clone(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    for i in resp.result.data.unwrap().account_list.into_iter() {
        let to: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Dotbit,
            identity: i.account.to_string(),
            uid: None,
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

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Dotbit,
            system: DomainNameSystem::DotBit,
            name: i.account.to_string(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };

        // hold record
        create_identity_to_identity_hold_record(&cli, &from, &to, &hold).await?;
        // 'regular' resolution involves mapping from a name to an address.
        create_identity_domain_resolve_record(&cli, &to, &from, &resolve).await?;
    }

    Ok(vec![])
}

fn get_req_params(coin_type: &CoinType, identity: &str) -> RequestTypeKeyInfoParams {
    // will support other platform later
    let req_key_info: RequestKeyInfo = RequestKeyInfo {
        coin_type: coin_type.to_string(),
        chain_id: "1".to_string(),
        key: identity.to_string().to_lowercase(),
    };
    return RequestTypeKeyInfoParams {
        req_type: "blockchain".to_string(),
        key_info: req_key_info,
    };
}

#[derive(
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    Debug,
    EnumIter,
    PartialEq,
    Eq,
    Hash,
)]
// 60: ETH, 195: TRX, 9006: BNB, 966: Matic, 3: doge, 309: ckb
pub enum CoinType {
    #[strum(serialize = "60")]
    #[serde(rename = "60")]
    ETH,

    #[strum(serialize = "195")]
    #[serde(rename = "195")]
    TRX,

    #[strum(serialize = "9006")]
    #[serde(rename = "9006")]
    BNB,

    #[strum(serialize = "966")]
    #[serde(rename = "966")]
    Matic,

    #[strum(serialize = "3")]
    #[serde(rename = "3")]
    Doge,

    #[strum(serialize = "309")]
    #[serde(rename = "309")]
    CKB,

    #[default]
    #[serde(rename = "unknown")]
    #[strum(serialize = "unknown")]
    Unknown,
}

impl From<Platform> for CoinType {
    fn from(platform: Platform) -> Self {
        match platform {
            Platform::Ethereum => CoinType::ETH,
            Platform::Tron => CoinType::TRX,
            Platform::BNBSmartChain => CoinType::BNB,
            Platform::Polygon => CoinType::Matic,
            Platform::Doge => CoinType::Doge,
            Platform::CKB => CoinType::CKB,
            _ => CoinType::Unknown,
        }
    }
}

impl From<i64> for Platform {
    fn from(algo_id: i64) -> Self {
        match algo_id {
            5 => Platform::Ethereum, // EIP712 = 5
            3 => Platform::Ethereum, // ETH = 3
            4 => Platform::Tron,     // TRX = 4
            8 => Platform::CKB,      // CKB = 8
            _ => Platform::Unknown,
        }
    }
}
