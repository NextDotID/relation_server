mod tests;
use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{Hold, HyperEdge, PartOfCollection, Proof, Resolve, Wrapper};
use crate::tigergraph::edge::{
    HOLD_IDENTITY, HYPER_EDGE, PART_OF_COLLECTION, PROOF_EDGE, RESOLVE, REVERSE_RESOLVE,
};
use crate::tigergraph::vertex::{DomainCollection, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, DomainSearch, DomainStatus, Fetcher, Platform,
    ProofLevel, TargetProcessedList, EXT,
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
            | Platform::Ton
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
            Platform::Ton,
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
        let updated_at_naive = timestamp_to_naive(d.updated_at, 0);
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
            created_at: updated_at_naive,
            display_name: Some(d.cluster_name.clone()),
            added_at: naive_now(),
            avatar_url: d.imageurl.clone(),
            profile_url: d.profileurl.clone(),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let clusters_name_node: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Clusters,
            identity: d.name.clone(),
            uid: None,
            created_at: updated_at_naive,
            display_name: Some(d.name.clone()),
            added_at: naive_now(),
            avatar_url: d.imageurl.clone(),
            profile_url: d.profileurl.clone(),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
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

        let clusters_connect = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: updated_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let proof_forward = Proof {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            level: ProofLevel::VeryConfident,
            record_id: None,
            created_at: updated_at_naive,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
        };

        let parent_node_hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            transaction: Some("".to_string()),
            id: d.cluster_name.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: None,
        };

        let child_node_hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            transaction: Some("".to_string()),
            id: d.name.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: None,
        };

        let parent_node_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.cluster_name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let child_node_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let parent_reverse_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.cluster_name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let child_reverse_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let pf = clusters_connect.wrapper(&clusters_parent_node, &clusters_name_node, PROOF_EDGE);
        let pf2 = proof_forward.wrapper(&clusters_name_node, &wallet, PROOF_EDGE);
        let parent_hd = parent_node_hold.wrapper(&wallet, &clusters_parent_node, HOLD_IDENTITY);
        let child_hd = child_node_hold.wrapper(&wallet, &clusters_name_node, HOLD_IDENTITY);
        let parent_rs = parent_node_resolve.wrapper(&clusters_parent_node, &wallet, RESOLVE);
        let child_rs = child_node_resolve.wrapper(&clusters_name_node, &wallet, RESOLVE);
        let parent_rr =
            parent_reverse_resolve.wrapper(&wallet, &clusters_parent_node, REVERSE_RESOLVE);
        let child_rr = child_reverse_resolve.wrapper(&wallet, &clusters_name_node, REVERSE_RESOLVE);

        edges.push(EdgeWrapperEnum::new_proof_forward(pf));
        edges.push(EdgeWrapperEnum::new_proof_backward(pf2));

        edges.push(EdgeWrapperEnum::new_hold_identity(parent_hd));
        edges.push(EdgeWrapperEnum::new_hold_identity(child_hd));

        edges.push(EdgeWrapperEnum::new_resolve(parent_rs));
        edges.push(EdgeWrapperEnum::new_resolve(child_rs));

        edges.push(EdgeWrapperEnum::new_reverse_resolve(parent_rr));
        edges.push(EdgeWrapperEnum::new_reverse_resolve(child_rr));
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
            avatar_url: d.imageurl.clone(),
            profile_url: d.profileurl.clone(),
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
            avatar_url: d.imageurl.clone(),
            profile_url: d.profileurl.clone(),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
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

        let parent_node_hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            transaction: Some("".to_string()),
            id: d.cluster_name.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: None,
        };

        let child_node_hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            transaction: Some("".to_string()),
            id: d.name.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: None,
        };

        let parent_node_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.cluster_name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let child_node_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let parent_reverse_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.cluster_name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let child_reverse_resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Clusters,
            system: DomainNameSystem::Clusters,
            name: d.name.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let pf = clusters_connect.wrapper(&clusters_parent_node, &clusters_name_node, PROOF_EDGE);
        let pf2 = proof_forward.wrapper(&clusters_name_node, &wallet, PROOF_EDGE);
        let parent_hd = parent_node_hold.wrapper(&wallet, &clusters_parent_node, HOLD_IDENTITY);
        let child_hd = child_node_hold.wrapper(&wallet, &clusters_name_node, HOLD_IDENTITY);
        let parent_rs = parent_node_resolve.wrapper(&clusters_parent_node, &wallet, RESOLVE);
        let child_rs = child_node_resolve.wrapper(&clusters_name_node, &wallet, RESOLVE);
        let parent_rr =
            parent_reverse_resolve.wrapper(&wallet, &clusters_parent_node, REVERSE_RESOLVE);
        let child_rr = child_reverse_resolve.wrapper(&wallet, &clusters_name_node, REVERSE_RESOLVE);

        edges.push(EdgeWrapperEnum::new_proof_forward(pf));
        edges.push(EdgeWrapperEnum::new_proof_backward(pf2));

        edges.push(EdgeWrapperEnum::new_hold_identity(parent_hd));
        edges.push(EdgeWrapperEnum::new_hold_identity(child_hd));

        edges.push(EdgeWrapperEnum::new_resolve(parent_rs));
        edges.push(EdgeWrapperEnum::new_resolve(child_rs));

        edges.push(EdgeWrapperEnum::new_reverse_resolve(parent_rr));
        edges.push(EdgeWrapperEnum::new_reverse_resolve(child_rr));

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
    #[serde(rename = "updatedat")]
    pub profileurl: Option<String>,
    pub imageurl: Option<String>,
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

#[async_trait]
impl DomainSearch for Clusters {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        let name = name.trim_end_matches("/");
        if name == "".to_string() {
            warn!("Clusters domain_search(name='') is not a valid handle name");
            return Ok(vec![]);
        }
        debug!("Clusters domain_search(name={})", name);

        let metadatas = get_address_by_clusters(name).await?;
        if metadatas.is_empty() {
            debug!("Clusters domain_search(name={}) result is empty", name);
            return Ok(vec![]);
        }
        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            id: name.to_string(),
            updated_at: naive_now(),
        };
        for d in metadatas.into_iter() {
            let wallet_platform: Platform = d.platform.parse()?;
            if wallet_platform == Platform::Unknown {
                warn!(
                    "Clusters domain_search(name={}) platform={} is Unknown in types",
                    d.cluster_name, d.platform
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
                avatar_url: d.imageurl.clone(),
                profile_url: d.profileurl.clone(),
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
                avatar_url: d.imageurl.clone(),
                profile_url: d.profileurl.clone(),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let parent_node_hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                transaction: Some("".to_string()),
                id: d.cluster_name.clone(),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::DataMgrService,
                expired_at: None,
            };

            let child_node_hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                transaction: Some("".to_string()),
                id: d.name.clone(),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::DataMgrService,
                expired_at: None,
            };

            let parent_node_resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                system: DomainNameSystem::Clusters,
                name: d.cluster_name.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };

            let child_node_resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                system: DomainNameSystem::Clusters,
                name: d.name.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };

            let parent_reverse_resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                system: DomainNameSystem::Clusters,
                name: d.cluster_name.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };

            let child_reverse_resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Clusters,
                system: DomainNameSystem::Clusters,
                name: d.name.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };

            let parent_collection_edge = PartOfCollection {
                platform: Platform::Clusters,
                name: d.cluster_name.clone(),
                tld: EXT::ClustersRoot.to_string(),
                status: DomainStatus::Taken,
            };
            let child_collection_edge = PartOfCollection {
                platform: Platform::Clusters,
                name: d.name.clone(),
                tld: d.name.split("/").last().unwrap_or("").to_string(),
                status: DomainStatus::Taken,
            };

            let parent_hd = parent_node_hold.wrapper(&wallet, &clusters_parent_node, HOLD_IDENTITY);
            let child_hd = child_node_hold.wrapper(&wallet, &clusters_name_node, HOLD_IDENTITY);
            let parent_rs = parent_node_resolve.wrapper(&clusters_parent_node, &wallet, RESOLVE);
            let child_rs = child_node_resolve.wrapper(&clusters_name_node, &wallet, RESOLVE);
            let parent_rr =
                parent_reverse_resolve.wrapper(&wallet, &clusters_parent_node, REVERSE_RESOLVE);
            let child_rr =
                child_reverse_resolve.wrapper(&wallet, &clusters_name_node, REVERSE_RESOLVE);
            let parent_c = parent_collection_edge.wrapper(
                &domain_collection,
                &clusters_parent_node,
                PART_OF_COLLECTION,
            );
            let child_c = child_collection_edge.wrapper(
                &domain_collection,
                &clusters_name_node,
                PART_OF_COLLECTION,
            );

            edges.push(EdgeWrapperEnum::new_hold_identity(parent_hd));
            edges.push(EdgeWrapperEnum::new_hold_identity(child_hd));

            edges.push(EdgeWrapperEnum::new_resolve(parent_rs));
            edges.push(EdgeWrapperEnum::new_resolve(child_rs));

            edges.push(EdgeWrapperEnum::new_reverse_resolve(parent_rr));
            edges.push(EdgeWrapperEnum::new_reverse_resolve(child_rr));

            edges.push(EdgeWrapperEnum::new_domain_collection_edge(parent_c));
            edges.push(EdgeWrapperEnum::new_domain_collection_edge(child_c));
        }

        Ok(edges)
    }
}
