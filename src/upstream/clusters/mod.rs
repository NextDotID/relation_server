mod tests;
use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{HyperEdge, Proof, Wrapper};
use crate::tigergraph::edge::{HYPER_EDGE, PROOF_EDGE};
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
use tracing::{debug, error, warn};
use uuid::Uuid;

use super::Target;

pub struct Clusters {}

#[async_trait]
impl Fetcher for Clusters {
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

        match target.platform()? {
            Platform::Ethereum
            | Platform::Solana
            | Platform::Bitcoin
            | Platform::Aptos
            | Platform::Doge
            | Platform::Near
            | Platform::Stacks
            | Platform::Tron
            | Platform::Xrpc
            | Platform::Cosmos => batch_fetch_by_address(target).await,
            Platform::Clusters => batch_fetch_by_clusters(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![
            Platform::Ethereum,
            Platform::Solana,
            Platform::Bitcoin,
            Platform::Aptos,
            Platform::Doge,
            Platform::Near,
            Platform::Stacks,
            Platform::Tron,
            Platform::Xrpc,
            Platform::Cosmos,
            Platform::Clusters,
        ])
    }
}

async fn batch_fetch_by_address(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let platform = target.platform()?;
    let mut address = target.identity()?;
    if platform == Platform::Ethereum {
        address = address.to_lowercase();
    }

    let metadatas = get_clusters_by_address(&address).await?;
    if metadatas.is_empty() {
        debug!(?target, "Clusters get_clusters_by_address result is empty");
        return Ok((vec![], vec![]));
    }

    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();
    for d in metadatas.into_iter() {
        let wallet_platform: Platform = d.platform.parse()?;
        if wallet_platform == Platform::Unknown {
            warn!(
                ?target,
                "Clusters platform({}) is Unknown in types", d.platform
            );
            continue;
        }
        let created_at_naive = timestamp_to_naive(d.updated_at, 0);
        let wallet: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: wallet_platform,
            identity: d.address.clone(),
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

        let clusters_parent_node: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Clusters,
            identity: d.cluster_name.clone(),
            uid: None,
            created_at: created_at_naive,
            display_name: Some(d.cluster_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let clusters_name_node: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Clusters,
            identity: d.name.clone(),
            uid: None,
            created_at: created_at_naive,
            display_name: Some(d.name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let clusters_connect = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: created_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let proof_forward = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: created_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        // add identity connected to hyper vertex
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &clusters_parent_node,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &clusters_name_node,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &wallet, HYPER_EDGE),
        ));

        let pf = clusters_connect.wrapper(&clusters_parent_node, &clusters_name_node, PROOF_EDGE);
        let pf2 = proof_forward.wrapper(&clusters_name_node, &wallet, PROOF_EDGE);

        edges.push(EdgeWrapperEnum::new_proof_forward(pf));
        edges.push(EdgeWrapperEnum::new_proof_backward(pf2));
    }

    Ok((vec![], edges))
}

async fn batch_fetch_by_clusters(
    target: &Target,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let name = target.identity()?.to_lowercase();
    let metadatas = get_address_by_clusters(&name).await?;
    if metadatas.is_empty() {
        debug!(?target, "Clusters get_address_by_clusters result is empty");
        return Ok((vec![], vec![]));
    }
    let mut next_targets = TargetProcessedList::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    for d in metadatas.into_iter() {
        let wallet_platform: Platform = d.platform.parse()?;
        if wallet_platform == Platform::Unknown {
            warn!(
                ?target,
                "Clusters platform({}) is Unknown in types", d.platform
            );
            continue;
        }
        let created_at_naive = timestamp_to_naive(d.updated_at, 0);
        let wallet: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: wallet_platform.clone(),
            identity: d.address.clone(),
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

        let clusters_parent_node: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Clusters,
            identity: d.cluster_name.clone(),
            uid: None,
            created_at: created_at_naive,
            display_name: Some(d.cluster_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let clusters_name_node: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Clusters,
            identity: d.name.clone(),
            uid: None,
            created_at: created_at_naive,
            display_name: Some(d.name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let clusters_connect = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: created_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let proof_forward = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: created_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        // add identity connected to hyper vertex
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &clusters_parent_node,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &clusters_name_node,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &wallet, HYPER_EDGE),
        ));

        let pf = clusters_connect.wrapper(&clusters_parent_node, &clusters_name_node, PROOF_EDGE);
        let pf2 = proof_forward.wrapper(&clusters_name_node, &wallet, PROOF_EDGE);

        edges.push(EdgeWrapperEnum::new_proof_forward(pf));
        edges.push(EdgeWrapperEnum::new_proof_backward(pf2));

        next_targets.push(Target::Identity(wallet_platform, d.address.clone()));
    }

    Ok((next_targets, edges))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetClustersResponse {
    pub code: i32,
    pub msg: String,
    pub data: Option<Vec<Metadata>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub address: String,
    pub platform: String,
    #[serde(rename = "clustername")]
    pub cluster_name: String,
    pub name: String,
    #[serde(rename = "isverified")]
    pub is_verified: bool,
    #[serde(rename = "updatedat")]
    pub updated_at: i64,
}

async fn get_clusters_by_address(address: &str) -> Result<Vec<Metadata>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/get_name?address={}",
        C.upstream.clusters_api.url.clone(),
        address
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("Clusters get_name Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Clusters get_name | error: {:?}",
                err.to_string()
            ))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("Clusters get_name error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<GetClustersResponse>(&mut resp).await {
        Ok(result) => {
            if result.code != 0 {
                let err_message = format!(
                    "Clusters get_name error | Code: {:?}, Message: {:?}",
                    result.code, result.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let r: Vec<Metadata> = result.data.map_or(vec![], |res| res);
            debug!("Clusters get_name records found {}.", r.len(),);
            r
        }
        Err(err) => {
            let err_message = format!("Clusters get_name error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    Ok(result)
}

async fn get_address_by_clusters(name: &str) -> Result<Vec<Metadata>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/get_address?name={}",
        C.upstream.clusters_api.url.clone(),
        name
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("Clusters get_address Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Clusters get_address | error: {:?}",
                err.to_string()
            ))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("Clusters get_address error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<GetClustersResponse>(&mut resp).await {
        Ok(result) => {
            if result.code != 0 {
                let err_message = format!(
                    "Clusters get_address error | Code: {:?}, Message: {:?}",
                    result.code, result.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let r: Vec<Metadata> = result.data.map_or(vec![], |res| res);
            debug!("Clusters get_address records found {}.", r.len(),);
            r
        }
        Err(err) => {
            let err_message = format!("Clusters get_address error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    Ok(result)
}
