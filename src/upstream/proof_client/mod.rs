extern crate futures;
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::{edge::Proof, new_db_connection, vertex::Identity};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform, Target, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, timestamp_to_naive};

use async_trait::async_trait;
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

use super::DataFetcher;

/// https://github.com/nextdotid/proof-server/blob/master/docs/api.apib
#[derive(Deserialize, Debug)]
pub struct ProofQueryResponse {
    pub pagination: ProofQueryResponsePagination,
    pub ids: Vec<ProofPersona>,
}

#[derive(Deserialize, Debug)]
pub struct ProofPersona {
    pub avatar: String,
    pub proofs: Vec<ProofRecord>,
}

#[derive(Deserialize, Debug)]
pub struct ProofRecord {
    pub platform: String,
    pub identity: String,
    pub created_at: String,
    pub last_checked_at: String,
    pub is_valid: bool,
    pub invalid_reason: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofQueryResponsePagination {
    pub total: u32,
    pub per: u32,
    pub current: u32,
    pub next: u32,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct ProofClient {}

#[async_trait]
impl Fetcher for ProofClient {
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
            Platform::Ethereum,
            Platform::Twitter,
            Platform::NextID,
            Platform::Github,
        ])
    }
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();

    let uri: http::Uri = format!(
        "{}/v1/proof?platform={}&identity={}",
        C.upstream.proof_service.url, platform, identity
    )
    .parse()
    .map_err(|_err| Error::ParamError("Uri format Error".to_string()))?;
    let mut resp = client.get(uri).await?;

    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        return Err(Error::General(
            format!("Proof Result Get Error: {}", body.message),
            resp.status(),
        ));
    }

    let mut body: ProofQueryResponse = parse_body(&mut resp).await?;
    if body.pagination.total == 0 {
        return Err(Error::NoResult);
    }

    let proofs = match body.ids.pop() {
        Some(i) => i,
        None => {
            return Err(Error::NoResult);
        }
    };
    let next_id_identity = proofs.avatar;
    let db = new_db_connection().await?;
    let mut next_targets: TargetProcessedList = vec![];

    for p in proofs.proofs.into_iter() {
        let from: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::NextID,
            identity: next_id_identity.clone(),
            created_at: Some(timestamp_to_naive(
                p.created_at.to_string().parse::<i64>().unwrap(),
                0,
            )),
            display_name: Some(next_id_identity.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
        };

        let from_record = from.create_or_update(&db).await?;
        let to_platform = Platform::from_str(p.platform.as_str());
        if to_platform.is_err() {
            continue;
        }

        let to: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: to_platform.unwrap(),
            identity: p.identity.to_string(),
            created_at: Some(timestamp_to_naive(
                p.created_at.to_string().parse().unwrap(),
                0,
            )),
            display_name: Some(p.identity.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
        };
        let to_record = to.create_or_update(&db).await?;

        next_targets.push(Target::Identity(to_platform.unwrap(), p.identity));

        let pf: Proof = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::NextID,
            record_id: None,
            created_at: Some(timestamp_to_naive(
                p.created_at.to_string().parse().unwrap(),
                0,
            )),
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };
        pf.connect(&db, &from_record, &to_record).await?;
    }
    Ok(next_targets)
}
