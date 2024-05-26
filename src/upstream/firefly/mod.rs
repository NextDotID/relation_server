#[cfg(test)]
mod tests;
use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{HyperEdge, Proof, Wrapper};
use crate::tigergraph::edge::{HYPER_EDGE, PROOF_EDGE, PROOF_REVERSE_EDGE};
use crate::tigergraph::vertex::{IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    DataFetcher, DataSource, Fetcher, Platform, ProofLevel, TargetProcessedList,
};
use crate::util::{make_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive};

use async_trait::async_trait;
use http::uri::InvalidUri;
use http::StatusCode;
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, error};
use uuid::Uuid;

use super::Target;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggregationResponse {
    pub code: i32,
    pub msg: Option<String>,
    data: Option<Vec<AggregationRecord>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggregationRecord {
    pub account_id: String,  // Firefly Account ID
    pub uid: Option<String>, // ID of the identity
    pub platform: String,    // Platform in [ethereum, farcaster, twitter]
    pub identity: String,
    pub data_source: String,  // firefly or admin(manually_added)
    pub update_time: i64,     // record update time
    pub display_name: String, // fname
}

pub struct Firefly {}

#[async_trait]
impl Fetcher for Firefly {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        Ok(vec![])
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        match target {
            Target::Identity(platform, identity) => {
                batch_fetch_connections(platform, identity).await
            }
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![
            Platform::Ethereum,
            Platform::Twitter,
            Platform::Farcaster,
        ])
    }
}

async fn batch_fetch_connections(
    platform: &Platform,
    identity: &str,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let records = search_records(platform, identity).await?;
    if records.is_empty() {
        debug!("Aggregation search result is empty");
        return Ok((vec![], vec![]));
    }
    debug!("Aggregation search records found {}.", records.len(),);
    let mut next_targets: Vec<Target> = Vec::new();
    let mut edges: Vec<EdgeWrapperEnum> = Vec::new();
    let hv = IdentitiesGraph::default();

    for (from_idx, from_v) in records.iter().enumerate() {
        let mut data_source = DataSource::Firefly;
        if from_v.data_source == String::from("admin") {
            data_source = DataSource::ManuallyAdded;
        }
        let from_update_naive = timestamp_to_naive(from_v.update_time, 0);
        let from_platform =
            Platform::from_str(from_v.platform.as_str()).unwrap_or(Platform::Unknown);
        if from_platform == Platform::Unknown {
            continue;
        }
        if from_platform != *platform {
            // Do not push duplicate targets into fetchjob
            next_targets.push(Target::Identity(from_platform, from_v.identity.clone()))
        }
        let from = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: from_platform.clone(),
            identity: from_v.identity.clone(),
            uid: from_v.uid.clone(),
            created_at: from_update_naive,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        for (to_idx, to_v) in records.iter().enumerate() {
            if to_idx >= from_idx {
                continue;
            }
            let to_update_naive = timestamp_to_naive(to_v.update_time, 0);
            let to_platform =
                Platform::from_str(to_v.platform.as_str()).unwrap_or(Platform::Unknown);
            if to_platform == Platform::Unknown {
                continue;
            }
            let to = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: to_platform.clone(),
                identity: to_v.identity.clone(),
                uid: to_v.uid.clone(),
                created_at: to_update_naive.clone(),
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            // add identity connected to hyper vertex
            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &from, HYPER_EDGE),
            ));
            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &to, HYPER_EDGE),
            ));

            let proof_forward = Proof {
                uuid: Uuid::new_v4(),
                source: data_source,
                level: ProofLevel::VeryConfident,
                record_id: Some(from_v.account_id.clone()),
                created_at: to_update_naive.clone(),
                updated_at: naive_now(),
                fetcher: DataFetcher::DataMgrService,
            };

            let proof_backward = Proof {
                uuid: Uuid::new_v4(),
                source: data_source,
                level: ProofLevel::VeryConfident,
                record_id: Some(from_v.account_id.clone()),
                created_at: to_update_naive.clone(),
                updated_at: naive_now(),
                fetcher: DataFetcher::DataMgrService,
            };

            let pf = proof_forward.wrapper(&from, &to, PROOF_EDGE);
            let pb = proof_backward.wrapper(&to, &from, PROOF_REVERSE_EDGE);

            edges.push(EdgeWrapperEnum::new_proof_forward(pf));
            edges.push(EdgeWrapperEnum::new_proof_backward(pb));
        }
    }

    Ok((next_targets, edges))
}

async fn search_records(
    platform: &Platform,
    identity: &str,
) -> Result<Vec<AggregationRecord>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/aggregation/search?platform={}&identity={}",
        C.upstream.aggregation_service.url.clone(),
        platform.to_string(),
        identity
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("Aggregation search Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Aggregation search | error: {:?}",
                err.to_string()
            ))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("Aggregation search error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<AggregationResponse>(&mut resp).await {
        Ok(result) => {
            if result.code != 0 {
                let err_message = format!(
                    "Aggregation search error | Code: {:?}, Message: {:?}",
                    result.code, result.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let return_data: Vec<AggregationRecord> = result.data.map_or(vec![], |res| res);
            return_data
        }
        Err(err) => {
            let err_message = format!("Genome get_address error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    Ok(result)
}
