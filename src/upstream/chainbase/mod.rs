#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, HyperEdge, Resolve, Wrapper, HOLD_CONTRACT, HOLD_IDENTITY, HYPER_EDGE, RESOLVE,
    RESOLVE_CONTRACT,
};
use crate::tigergraph::upsert::create_contract_to_identity_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_hold_record;
use crate::tigergraph::upsert::{create_ens_identity_ownership, create_ens_identity_resolve};
use crate::tigergraph::vertex::{Contract, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target,
    TargetProcessedList,
};
use crate::util::{make_http_client, naive_now, parse_timestamp};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use gql_client::Client as GQLClient;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

pub struct Chainbase {}

#[async_trait]
impl Fetcher for Chainbase {
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

        // batch_perform_fetch(target).await
        Ok((vec![], vec![]))
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
            || target.in_nft_supported(vec![ContractCategory::ENS], vec![Chain::Ethereum])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveEnsDomainResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: ResolveDomain,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveDomain {
    /// ENS name (`something.eth`)
    name: String, // "" = null
    address: String,         // "" = null
    registrant: String,      // "" = null
    owner: String,           // "" = null
    resolver: String,        // "" = null
    registrant_time: String, // "0001-01-01T00:00:00Z"
    expiration_time: String, // "0001-01-01T00:00:00Z"
    token_id: String,        // "<nil>"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FireflyData {
    pub code: i32,
    pub data: Profile,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    #[serde(rename = "walletProfiles")]
    pub wallet_profiles: Vec<WalletProfile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletProfile {
    pub address: String,
    pub ens: Vec<String>,
    pub primary_ens: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Domain {
    /// ENS name (`something.eth`)
    name: String,
    created_at: Option<NaiveDateTime>,
    expired_at: Option<NaiveDateTime>,
    token_id: Option<String>,
    resolved_address: Option<String>,
    owner: String,
    reverse: bool,
}

async fn fetch_domain(target: &Target) -> Result<Vec<Domain>, Error> {
    todo!()
}
