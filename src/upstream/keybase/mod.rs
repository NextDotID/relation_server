#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::Proof;
use crate::tigergraph::upsert::create_identity_to_identity_proof_two_way_binding;
use crate::tigergraph::vertex::{Identity, IdentityRecord};
use crate::tigergraph::{BaseResponse, Graph};
use crate::upstream::{DataSource, Fetcher, Platform, ProofLevel, TargetProcessedList};
use crate::util::{
    make_client, make_http_client, naive_now, option_naive_datetime_from_string,
    option_naive_datetime_to_string, parse_body, request_with_timeout,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use http::uri::InvalidUri;
use http::StatusCode;
use hyper::{Body, Method};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, error};
use uuid::Uuid;

use super::{DataFetcher, Target};

#[derive(Deserialize, Debug)]
pub struct KeybaseResponse {
    pub status: Status,
    pub them: Vec<PersonInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PersonInfo {
    pub id: String,
    pub basics: Basics,
    pub proofs_summary: ProofsSummary,
}

#[derive(Deserialize, Debug)]
pub struct Status {
    pub code: i32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofsSummary {
    pub all: Vec<ProofItem>,
}

#[derive(Deserialize, Debug)]
pub struct Basics {
    pub username: String,
    pub ctime: i64,
    pub mtime: i64,
    pub id_version: i32,
    pub track_version: i32,
    pub last_id_change: i64,
    pub username_cased: String,
    pub status: i32,
    pub salt: String,
    pub eldest_seqno: i32,
}

#[derive(Deserialize, Debug)]
pub struct ProofItem {
    pub proof_type: String,
    pub nametag: String,
    pub state: i32,
    pub service_url: String,
    pub proof_url: String,
    pub sig_id: String,
    pub proof_id: String,
    pub human_url: String,
    pub presentation_group: String,
    pub presentation_tag: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryKeybaseConnectionsResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<KeybaseConnections>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct KeybaseConnections {
    vertices: Vec<IdentityRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct StableKeybaseResponse {
    code: i32,
    msg: String,
    data: Option<Vec<KeybaseProof>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct KeybaseProof {
    keybase_username: String,
    platform: String,
    username: String,
    display_name: Option<String>,
    proof_type: i32,
    proof_state: i32,
    record_id: Option<String>,
    #[serde(deserialize_with = "option_naive_datetime_from_string")]
    #[serde(serialize_with = "option_naive_datetime_to_string")]
    created_time: Option<NaiveDateTime>,
}

#[derive(Default)]
pub struct Keybase {}

#[async_trait]
impl Fetcher for Keybase {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(platform, identity) => {
                stable_fetch_connections_by_platform_identity(platform, identity).await
            }
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![
            Platform::Twitter,
            Platform::Github,
            Platform::Reddit,
            Platform::Keybase,
            Platform::DNS,
            Platform::MstdnJP,
            Platform::Lobsters,
            Platform::HackerNews,
            Platform::Facebook,
            Platform::Bitcoin,
        ])
    }
}

#[allow(dead_code)]
async fn fake_fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut next_targets: TargetProcessedList = Vec::new();

    let vid: String = format!("{},{}", platform.to_string(), identity.to_string());
    let encoded_vid = urlencoding::encode(&vid);

    let cli = make_http_client();
    let uri: http::Uri = format!(
        "{}/query/{}/query_keybase_connections?p={}",
        C.tdb.host,
        Graph::IdentityGraph.to_string(),
        encoded_vid,
    )
    .parse()
    .map_err(|_err: InvalidUri| {
        Error::ParamError(format!(
            "QUERY query_keybase_connections?p={} Uri format Error | {}",
            encoded_vid, _err
        ))
    })?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header("Authorization", Graph::IdentityGraph.token())
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("ParamError Error | {}", _err)))?;

    let mut resp = cli.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "query query_keybase_connections?p={} error | Fail to request: {:?}",
            encoded_vid,
            err.to_string()
        ))
    })?;

    let person_info = match parse_body::<QueryKeybaseConnectionsResponse>(&mut resp).await {
        Ok(r) => {
            if r.base.error {
                let err_message = format!(
                    "TigerGraph query query_keybase_connections error | Code: {:?}, Message: {:?}",
                    r.base.code, r.base.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
            let result = r
                .results
                .and_then(|results| results.first().cloned())
                .map(|keybase_res| keybase_res.vertices)
                .map_or(vec![], |res| {
                    res.into_iter()
                        .filter(|target| target.v_id() != vid)
                        .collect()
                });
            result
        }
        Err(err) => {
            let err_message = format!("TigerGraph query owned_by parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    let _ = person_info.iter().map(|info| {
        next_targets.push(Target::Identity(
            info.platform.clone(),
            info.identity.clone(),
        ))
    });

    Ok(next_targets)
}

async fn stable_fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut next_targets: TargetProcessedList = Vec::new();
    let client = make_http_client();

    // BTC Character case sensitive
    let mut format_identity = identity.to_string();
    if platform.to_owned() != Platform::Bitcoin {
        format_identity = format_identity.to_lowercase();
    }

    let uri: http::Uri = format!(
        "{}/proofs_summary?platform={}&username={}",
        C.upstream.keybase_service.stable_url, platform, format_identity
    )
    .parse()
    .map_err(|_err: InvalidUri| {
        Error::ParamError(format!(
            "{}={} Uri format Error | {}",
            platform, format_identity, _err
        ))
    })?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("ParamError Error | {}", _err)))?;

    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "Keybase proofs_summary?platform={}&identity={} error | Fail to request: {:?}",
            platform,
            format_identity,
            err.to_string()
        ))
    })?;

    let proofs = match parse_body::<StableKeybaseResponse>(&mut resp).await {
        Ok(r) => {
            if r.code != 0 {
                let err_message = format!(
                    "Keybase proofs_summary error | Code: {:?}, Message: {:?}",
                    r.code, r.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let result = r.data.map_or(vec![], |res| res);
            tracing::debug!("proofs_summary result {:?}", result);
            debug!("Keybase proofs_summary = {} Records found.", result.len(),);
            result
        }
        Err(err) => {
            let err_message = format!("Keybase proofs_summary error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    let cli = make_http_client(); // db connection
    for p in proofs.into_iter() {
        let to_platform = Platform::from_str(&p.platform.as_str()).unwrap_or(Platform::Unknown);
        if to_platform == Platform::Unknown {
            continue;
        }
        let from: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Keybase,
            identity: p.keybase_username.clone(),
            uid: None,
            created_at: None,
            display_name: Some(p.keybase_username.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let to: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: to_platform.clone(),
            identity: p.username.clone(),
            uid: None,
            created_at: None,
            display_name: p.display_name,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let pf: Proof = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Keybase,
            level: ProofLevel::VeryConfident,
            record_id: p.record_id.clone(),
            created_at: p.created_time.clone(),
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };

        let pb: Proof = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Keybase,
            level: ProofLevel::VeryConfident,
            record_id: p.record_id.clone(),
            created_at: p.created_time.clone(),
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };

        create_identity_to_identity_proof_two_way_binding(&cli, &from, &to, &pf, &pb).await?;

        next_targets.push(Target::Identity(to_platform, p.username.clone()));
    }

    Ok(next_targets)
}

#[allow(dead_code)]
async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();
    let uri: http::Uri = match format!(
        "{}?{}={}&fields=proofs_summary",
        C.upstream.keybase_service.url, platform, identity
    )
    .parse()
    {
        Ok(n) => n,
        Err(err) => return Err(Error::ParamError(format!("Uri format Error: {}", err))),
    };

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Keybase Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("Keybase fetch | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        return Err(Error::General(
            format!("Keybase Result Get Error: {}", body.message),
            resp.status(),
        ));
    }

    let mut body: KeybaseResponse = parse_body(&mut resp).await?;
    if body.status.code != 0 {
        return Err(Error::General(
            format!("Keybase Result Get Error: {}", body.status.name),
            resp.status(),
        ));
    }

    let person_info = match body.them.pop() {
        Some(i) => i,
        None => {
            return Err(Error::NoResult);
        }
    };
    let user_id = person_info.id;
    let user_name = person_info.basics.username;
    let cli = make_http_client();
    let mut next_targets: TargetProcessedList = Vec::new();

    for p in person_info.proofs_summary.all.into_iter() {
        let from: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Keybase,
            identity: user_id.clone(),
            uid: None,
            created_at: None,
            display_name: Some(user_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        if Platform::from_str(p.proof_type.as_str()).is_err() {
            continue;
        }
        let to: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::from_str(p.proof_type.as_str()).unwrap(),
            identity: p.nametag.clone().to_lowercase(),
            uid: None,
            created_at: None,
            display_name: Some(p.nametag.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let pf: Proof = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Keybase,
            level: ProofLevel::VeryConfident,
            record_id: Some(p.proof_id.clone()),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };

        let pb: Proof = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Keybase,
            level: ProofLevel::VeryConfident,
            record_id: Some(p.proof_id.clone()),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };

        create_identity_to_identity_proof_two_way_binding(&cli, &from, &to, &pf, &pb).await?;

        next_targets.push(Target::Identity(
            Platform::from_str(&p.proof_type).unwrap(),
            p.nametag,
        ));
    }

    Ok(next_targets)
}
