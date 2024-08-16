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
use crate::util::{make_client, naive_now, parse_body, request_with_timeout};

use async_trait::async_trait;
use http::uri::InvalidUri;
use http::StatusCode;
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use uuid::Uuid;

use super::Target;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataResponse {
    pub code: i32,
    pub msg: Option<String>,
    pub data: Option<Vec<SnsRecord>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnsRecord {
    pub address: String,      // evm address
    pub sns_platform: String, // Platform in [twitter, instagram]
    pub sns_handle: String,
    pub is_verified: bool, // is verified or not
}

pub struct OpenSea {}

#[async_trait]
impl Fetcher for OpenSea {
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
            Platform::Instagram,
        ])
    }
}

async fn batch_fetch_connections(
    platform: &Platform,
    identity: &str,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let records = search_opensea_account(platform, identity).await?;
    if records.is_empty() {
        debug!("OpenSea search result is empty");
        return Ok((vec![], vec![]));
    }
    debug!("OpenSea search records found {}.", records.len(),);
    let mut next_targets: Vec<Target> = Vec::new();
    let mut edges: Vec<EdgeWrapperEnum> = Vec::new();
    let hv = IdentitiesGraph::default();

    for record in records.iter() {
        let sns_platform: Platform = record.sns_platform.parse().unwrap_or(Platform::Unknown);
        let sns_handle = record.sns_handle.clone();
        let address = record.address.clone();
        if !record.is_verified {
            debug!(
                "OpenSea search address({}) => {}={} not verified.",
                address, sns_platform, sns_handle
            );
            continue;
        }

        if sns_platform == Platform::Unknown {
            continue;
        }

        if sns_platform != *platform {
            // Do not push duplicate targets into fetchjob
            next_targets.push(Target::Identity(sns_platform, sns_handle.clone()))
        }

        let from = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: address,
            uid: None,
            created_at: None,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let to = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: sns_platform,
            identity: sns_handle.clone(),
            uid: None,
            created_at: None,
            display_name: Some(sns_handle.clone()),
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
            source: DataSource::OpenSea,
            level: ProofLevel::Confident,
            record_id: None,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let proof_backward = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::OpenSea,
            level: ProofLevel::Confident,
            record_id: None,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let pf = proof_forward.wrapper(&from, &to, PROOF_EDGE);
        let pb = proof_backward.wrapper(&to, &from, PROOF_REVERSE_EDGE);

        edges.push(EdgeWrapperEnum::new_proof_forward(pf));
        edges.push(EdgeWrapperEnum::new_proof_backward(pb));
    }

    Ok((next_targets, edges))
}

async fn search_opensea_account(
    platform: &Platform,
    identity: &str,
) -> Result<Vec<SnsRecord>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/aggregation/opensea_account?platform={}&identity={}",
        C.upstream.aggregation_service.url.clone(),
        platform.to_string(),
        identity
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("OpenSea Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("OpenSea search Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("OpenSea search | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("OpenSea search error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<DataResponse>(&mut resp).await {
        Ok(result) => {
            if result.code != 0 {
                let err_message = format!(
                    "OpenSea search error | Code: {:?}, Message: {:?}",
                    result.code, result.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let return_data: Vec<SnsRecord> = result.data.map_or(vec![], |res| res);
            return_data
        }
        Err(err) => {
            let err_message = format!("OpenSea search parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    Ok(result)
}
