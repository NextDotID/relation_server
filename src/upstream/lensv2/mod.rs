#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{Hold, Resolve};
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_identity_hold_record;
use crate::tigergraph::vertex::Identity;
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{
    make_client, make_http_client, naive_now, option_naive_datetime_from_utc_string, parse_body,
};
use async_trait::async_trait;
use cynic::QueryFragment;
use cynic::{http::SurfExt, QueryBuilder};
use http::uri::InvalidUri;
use hyper::body;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

mod schema {
    cynic::use_schema!("src/upstream/lensv2/schema.graphql");
}

// Query by Handles
#[derive(cynic::QueryVariables, Debug, Default)]
pub struct ProfilesRequestVariables {
    pub handles: Option<Vec<Handle>>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "Query",
    schema_path = "src/upstream/lensv2/schema.graphql",
    variables = "ProfilesRequestVariables"
)]
pub struct ProfileQueryByHandles {
    #[arguments(request: { where: { handles: $handles}} )]
    pub profiles: PaginatedProfileResult,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct PaginatedProfileResult {
    pub items: Vec<Profile>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct Profile {
    pub id: ProfileId,
    pub handle: Option<HandleInfo>,
    pub created_at: DateTime,
    pub owned_by: NetworkAddress,
    pub metadata: Option<ProfileMetadata>,
    pub tx_hash: TxHash,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct ProfileMetadata {
    pub display_name: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct NetworkAddress {
    pub address: EvmAddress,
    pub chain_id: ChainId,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct HandleInfo {
    pub id: TokenId,
    pub full_handle: Handle,
    pub local_name: String,
    pub namespace: String,
    pub owned_by: EvmAddress,
}

#[derive(cynic::Scalar, Debug, Clone)]
pub struct ChainId(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct DateTime(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct EvmAddress(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct Handle(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct ProfileId(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct TokenId(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
pub struct TxHash(pub String);

pub struct LensV2 {}

#[async_trait]
impl Fetcher for LensV2 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target.platform()? {
            Platform::Ethereum => fetch_by_wallet(target).await,
            Platform::Lens => fetch_by_lens_handle(target).await,
            _ => Ok(vec![]),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Lens])
    }
}

async fn fetch_by_lens_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    let target_var = target.identity()?;
    let handle = target_var.trim_end_matches(".lens");
    let full_handle = format!("lens/{}", handle);
    let operation = ProfileQueryByHandles::build(ProfilesRequestVariables {
        handles: Some(vec![Handle(full_handle)]),
    });
    println!("{}", operation.query);
    let response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(operation)
        .await;
    if response.is_err() {
        warn!(
            "LensV2 target {} | Failed to fetch: {}",
            target,
            response.unwrap_err(),
        );
        return Ok(vec![]);
    }
    info!("Lens response {:?}", response);
    Ok(vec![])
}

async fn fetch_by_wallet(target: &Target) -> Result<TargetProcessedList, Error> {
    todo!()
}
