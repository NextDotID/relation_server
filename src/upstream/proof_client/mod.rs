extern crate futures;
#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::graph::{edge::Proof, new_db_connection, vertex::Identity};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform, Target, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive};

use async_trait::async_trait;
use hyper::{Body, Method};
use serde::Deserialize;
use std::str::FromStr;
use tracing::{debug, error, event, span, Level, Instrument};
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
            Platform::Dotbit,
        ])
    }
}

#[tracing::instrument(level = "trace", fields(platform = %platform, identity = %identity))]
async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();

    let uri: http::Uri = format!(
        "{}/v1/proof?exact=true&platform={}&identity={}",
        C.upstream.proof_service.url, platform, identity
    )
    .parse()
    .map_err(|_err| Error::ParamError("Uri format Error".to_string()))?;

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Proof Service Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "Proof Service fetch | error: {:?}",
            err.to_string()
        ))
    })?;

    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        error!("Proof Service fetch error, status {}", resp.status());
        return Err(Error::General(
            format!("Proof Result Get Error: {}", body.message),
            resp.status(),
        ));
    }

    let query_result: ProofQueryResponse = parse_body(&mut resp).await?;
    if query_result.pagination.total == 0 {
        return Err(Error::NoResult);
    }

    debug!(length = query_result.ids.len(), "Found.");
    if query_result.ids.len() == 0 {
        return Err(Error::NoResult);
    }

    let mut next_targets: TargetProcessedList = vec![];
    // let next_id_identity = proofs.avatar;
    let db = new_db_connection().await?;
    for id in query_result.ids {
        let ProofPersona { avatar, proofs } = id;

        for p in proofs.into_iter() {
            if p.is_valid == false {
                continue;
            }
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::NextID,
                identity: avatar.clone(),
                created_at: Some(timestamp_to_naive(
                    p.created_at.to_string().parse::<i64>().unwrap(),
                    0,
                )),
                display_name: Some(avatar.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };

            let from_record = from.create_or_update(&db).await?;
            let to_platform = Platform::from_str(p.platform.as_str()).unwrap_or(Platform::Unknown);
            if to_platform == Platform::Unknown {
                event!(
                    Level::WARN,
                    ?platform,
                    identity,
                    platform = p.platform,
                    "found unknown connected platform",
                );
                continue;
            }

            let to: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: to_platform,
                identity: p.identity.to_string().to_lowercase(),
                created_at: Some(timestamp_to_naive(
                    p.created_at.to_string().parse().unwrap(),
                    0,
                )),
                // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
                display_name: if to_platform == Platform::Ethereum {
                    None
                } else {
                    Some(p.identity.clone())
                },
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let to_record = to.create_or_update(&db).await?;

            next_targets.push(Target::Identity(to_platform, p.identity));

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
            pf.two_way_binding(&db, &from_record, &to_record).await?;
        }
    }
    next_targets.dedup();
    event!(Level::TRACE, "Next target count: {:?}", next_targets.len());
    Ok(next_targets)
}
