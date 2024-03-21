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
use crate::util::{make_http_client, naive_now, utc_to_naive};
use async_trait::async_trait;
use cynic::{http::SurfExt, QueryBuilder};
use hyper::{client::HttpConnector, Client};
use tracing::{trace, warn};
use uuid::Uuid;

mod schema {
    cynic::use_schema!("src/upstream/lensv2/schema.graphql");
}

// Query by Handles
#[derive(cynic::QueryVariables, Debug, Default)]
pub struct ProfilesRequestVariables {
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub handles: Option<Vec<Handle>>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    #[cynic(rename = "ownedBy")]
    pub owned_by: Option<Vec<EvmAddress>>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "Query",
    schema_path = "src/upstream/lensv2/schema.graphql",
    variables = "ProfilesRequestVariables"
)]
pub struct ProfileQueryByHandles {
    #[arguments(request: { where: { handles: $handles, ownedBy: $owned_by}} )]
    pub profiles: PaginatedProfileResult,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct PaginatedProfileResult {
    pub items: Vec<Profile>,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct Profile {
    pub id: ProfileId,
    pub handle: Option<HandleInfo>,
    pub created_at: DateTime,
    pub owned_by: NetworkAddress,
    pub metadata: Option<ProfileMetadata>,
    pub tx_hash: TxHash,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct ProfileMetadata {
    pub display_name: Option<String>,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct NetworkAddress {
    pub address: EvmAddress,
    pub chain_id: ChainId,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct HandleInfo {
    pub id: TokenId,
    pub full_handle: Handle,
    pub local_name: String,
    pub namespace: String,
    pub owned_by: EvmAddress,
}

#[derive(cynic::Scalar, Debug, Clone)]
pub struct ChainId(pub u32);

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
    let handle_name = target_var.trim_end_matches(".lens");
    let full_handle = format!("lens/{}", handle_name);
    let operation = ProfileQueryByHandles::build(ProfilesRequestVariables {
        handles: Some(vec![Handle(full_handle.clone())]),
        owned_by: None,
    });
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
    let cli = make_http_client();
    let profiles = response
        .unwrap()
        .data
        .map_or(vec![], |data| data.profiles.items);

    let mut next_targets: Vec<Target> = Vec::new();
    for profile in profiles.iter() {
        let t = save_profile(&cli, profile).await?;
        if let Some(t) = t {
            next_targets.push(t);
        }
    }

    Ok(next_targets)
}

async fn fetch_by_wallet(target: &Target) -> Result<TargetProcessedList, Error> {
    let target_var = target.identity()?;
    let owned_by_evm = target_var.to_lowercase();
    let operation = ProfileQueryByHandles::build(ProfilesRequestVariables {
        handles: None,
        owned_by: Some(vec![EvmAddress(owned_by_evm.clone())]),
    });
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
    let cli = make_http_client();
    let profiles = response
        .unwrap()
        .data
        .map_or(vec![], |data| data.profiles.items);
    let mut next_targets: Vec<Target> = Vec::new();
    for profile in profiles.iter() {
        let t = save_profile(&cli, profile).await?;
        if let Some(t) = t {
            next_targets.push(t);
        }
    }

    Ok(next_targets)
}

async fn save_profile(
    client: &Client<HttpConnector>,
    profile: &Profile,
) -> Result<Option<Target>, Error> {
    if profile.handle.clone().is_none() {
        return Ok(None);
    }
    let handle_info = profile.handle.clone().unwrap();
    let owner = profile.owned_by.address.0.to_ascii_lowercase();
    let lens_handle = format!("{}.{}", handle_info.local_name, handle_info.namespace);
    let lens_display_name = profile
        .metadata
        .clone()
        .map_or(None, |metadata| metadata.display_name);
    let created_at = utc_to_naive(profile.created_at.clone().0)?;

    let addr: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: owner.clone(),
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

    let lens: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Lens,
        identity: lens_handle.clone(),
        uid: Some(profile.id.clone().0.to_string()),
        created_at: Some(created_at),
        display_name: lens_display_name,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: Some("https://hey.xyz/u/".to_owned() + &handle_info.local_name),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(true),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        transaction: Some(profile.tx_hash.clone().0),
        id: profile.id.clone().0.to_string(),
        created_at: Some(created_at),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        system: DomainNameSystem::Lens,
        name: lens_handle.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // field `is_default` has been canceled in lens-v2-api
    let reverse: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        system: DomainNameSystem::Lens,
        name: lens_handle.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };
    trace!("LensV2 Ethereum({}) handle: {}", owner, lens_handle);
    create_identity_to_identity_hold_record(client, &addr, &lens, &hold).await?;
    create_identity_domain_resolve_record(client, &lens, &addr, &resolve).await?;
    create_identity_domain_reverse_resolve_record(client, &addr, &lens, &reverse).await?;
    Ok(Some(Target::Identity(Platform::Ethereum, owner.clone())))
}
