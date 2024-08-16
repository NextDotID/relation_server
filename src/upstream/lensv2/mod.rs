#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, HyperEdge, PartOfCollection, Resolve, Wrapper, HOLD_IDENTITY, HYPER_EDGE,
    PART_OF_COLLECTION, RESOLVE, REVERSE_RESOLVE,
};
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_identity_hold_record;
use crate::tigergraph::vertex::{DomainCollection, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, DomainSearch, DomainStatus, Fetcher, Platform,
    Target, TargetProcessedList, EXT,
};
use crate::util::{make_http_client, naive_now, utc_to_naive};
use async_trait::async_trait;
use cynic::{http::SurfExt, QueryBuilder};
use hyper::{client::HttpConnector, Client};
use tracing::{debug, trace, warn};
use uuid::Uuid;

mod schema {
    cynic::use_schema!("src/upstream/lensv2/schema.graphql");
}

#[derive(cynic::QueryVariables, Debug)]
pub struct DefaultProfileVariables {
    pub evm_address: EvmAddress,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "Query",
    schema_path = "src/upstream/lensv2/schema.graphql",
    variables = "DefaultProfileVariables"
)]
pub struct GetDefaultProfile {
    #[arguments(request: { for: $evm_address })]
    pub default_profile: Option<Profile>,
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

#[allow(dead_code)]
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "src/upstream/lensv2/schema.graphql")]
pub struct NetworkAddress {
    pub address: EvmAddress,
    pub chain_id: ChainId,
}

#[allow(dead_code)]
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

impl PartialEq for ProfileId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

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

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        match target.platform()? {
            Platform::Ethereum => batch_fetch_by_wallet(target).await,
            Platform::Lens => batch_fetch_by_handle(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Lens])
    }
}

async fn query_by_handle(handle_name: &str) -> Result<Vec<Profile>, Error> {
    let operation = ProfileQueryByHandles::build(ProfilesRequestVariables {
        handles: Some(vec![Handle(handle_name.to_string())]),
        owned_by: None,
    });
    let response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(operation)
        .await;
    if response.is_err() {
        warn!(
            "LensV2 {} | Failed to fetch: {}",
            handle_name,
            response.unwrap_err(),
        );
        return Ok(vec![]);
    }

    let profiles = response
        .unwrap()
        .data
        .map_or(vec![], |data| data.profiles.items);

    Ok(profiles)
}

async fn query_by_wallet(wallet: &str) -> Result<Vec<Profile>, Error> {
    let operation = ProfileQueryByHandles::build(ProfilesRequestVariables {
        handles: None,
        owned_by: Some(vec![EvmAddress(wallet.to_string())]),
    });
    let response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(operation)
        .await;

    if response.is_err() {
        warn!(
            "LensV2 {} | Failed to fetch: {}",
            wallet,
            response.unwrap_err(),
        );
        return Ok(vec![]);
    }
    let profiles = response
        .unwrap()
        .data
        .map_or(vec![], |data| data.profiles.items);

    Ok(profiles)
}

async fn batch_fetch_by_handle(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let target_var = target.identity()?;
    let handle_name = target_var.trim_end_matches(".lens");
    let full_handle = format!("lens/{}", handle_name);
    let profiles = query_by_handle(&full_handle).await?;
    if profiles.is_empty() {
        warn!("LensV2 target {} | No Result", target,);
        return Ok((vec![], vec![]));
    }
    let lens_profile = profiles.first().unwrap().clone();
    if lens_profile.handle.clone().is_none() {
        warn!("LensV2 target {} | lens handle is null", target,);
        return Ok((vec![], vec![]));
    }

    let mut next_targets = TargetProcessedList::new();
    let hv = IdentitiesGraph::default();
    let mut edges = EdgeList::new();

    let evm_owner = lens_profile.owned_by.address.0.to_ascii_lowercase();
    // fetch default profile id for lens-v2
    let default_profile_id = get_default_profile_id(&evm_owner).await?;

    let mut is_default = false;
    if let Some(default_profile_id) = default_profile_id {
        if lens_profile.id == default_profile_id {
            trace!(
                "LensV2 target {} | profile.id {:?} == default_profile_id {:?}",
                target,
                lens_profile.id,
                default_profile_id
            );
            is_default = true;
        }
    }
    let handle_info = lens_profile.handle.clone().unwrap();
    let lens_handle = format!("{}.{}", handle_info.local_name, handle_info.namespace);
    let lens_display_name = lens_profile
        .metadata
        .clone()
        .map_or(None, |metadata| metadata.display_name);
    let created_at = utc_to_naive(lens_profile.created_at.clone().0)?;

    let addr: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: evm_owner.clone(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(is_default),
    };

    let lens: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Lens,
        identity: lens_handle.clone(),
        uid: Some(lens_profile.id.clone().0.to_string()),
        created_at: Some(created_at),
        display_name: lens_display_name,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: Some("https://hey.xyz/u/".to_owned() + &handle_info.local_name),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(is_default),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        transaction: Some(lens_profile.tx_hash.clone().0),
        id: lens_profile.id.clone().0.to_string(),
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

    if is_default {
        // field `is_default` has been canceled in lens-v2-api
        // It is an independent query `GetDefaultProfile` and is not returned in the profile field.
        let reverse: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Lens,
            system: DomainNameSystem::Lens,
            name: lens_handle.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        let rrs = reverse.wrapper(&addr, &lens, REVERSE_RESOLVE);
        edges.push(EdgeWrapperEnum::new_reverse_resolve(rrs));
    }

    edges.push(EdgeWrapperEnum::new_hyper_edge(
        HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
    ));
    edges.push(EdgeWrapperEnum::new_hyper_edge(
        HyperEdge {}.wrapper(&hv, &lens, HYPER_EDGE),
    ));

    let hd = hold.wrapper(&addr, &lens, HOLD_IDENTITY);
    let rs = resolve.wrapper(&lens, &addr, RESOLVE);
    edges.push(EdgeWrapperEnum::new_hold_identity(hd));
    edges.push(EdgeWrapperEnum::new_resolve(rs));

    next_targets.push(Target::Identity(Platform::Ethereum, evm_owner.clone()));

    Ok((next_targets, edges))
}

async fn batch_fetch_by_wallet(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let target_var = target.identity()?;
    let owned_by_evm = target_var.to_lowercase();
    let profiles = query_by_wallet(&owned_by_evm).await?;
    if profiles.is_empty() {
        warn!("LensV2 target {} | No Result", target,);
        return Ok((vec![], vec![]));
    }

    // fetch default profile id for lens-v2
    let default_profile_id = get_default_profile_id(&owned_by_evm).await?;
    let hv = IdentitiesGraph::default();
    let mut edges = EdgeList::new();

    for lens_profile in profiles.iter() {
        let mut is_default = false;
        if let Some(default_profile_id) = default_profile_id.clone() {
            if lens_profile.id == default_profile_id {
                trace!(
                    "LensV2 target {} | profile.id {:?} == default_profile_id {:?}",
                    target,
                    lens_profile.id,
                    default_profile_id
                );
                is_default = true;
            }
        }
        let evm_owner = lens_profile.owned_by.address.0.to_ascii_lowercase();
        let handle_info = lens_profile.handle.clone().unwrap();
        let lens_handle = format!("{}.{}", handle_info.local_name, handle_info.namespace);
        let lens_display_name = lens_profile
            .metadata
            .clone()
            .map_or(None, |metadata| metadata.display_name);
        let created_at = utc_to_naive(lens_profile.created_at.clone().0)?;

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: evm_owner.clone(),
            uid: None,
            created_at: None,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(is_default),
        };

        let lens: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Lens,
            identity: lens_handle.clone(),
            uid: Some(lens_profile.id.clone().0.to_string()),
            created_at: Some(created_at),
            display_name: lens_display_name,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: Some("https://hey.xyz/u/".to_owned() + &handle_info.local_name),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(is_default),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Lens,
            transaction: Some(lens_profile.tx_hash.clone().0),
            id: lens_profile.id.clone().0.to_string(),
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

        if is_default {
            // field `is_default` has been canceled in lens-v2-api
            // It is an independent query `GetDefaultProfile` and is not returned in the profile field.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Lens,
                system: DomainNameSystem::Lens,
                name: lens_handle.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };
            let rrs = reverse.wrapper(&addr, &lens, REVERSE_RESOLVE);
            edges.push(EdgeWrapperEnum::new_reverse_resolve(rrs));
        }

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
        ));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &lens, HYPER_EDGE),
        ));

        let hd = hold.wrapper(&addr, &lens, HOLD_IDENTITY);
        let rs = resolve.wrapper(&lens, &addr, RESOLVE);
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        edges.push(EdgeWrapperEnum::new_resolve(rs));
    }

    Ok((vec![], edges))
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
        if profile.handle.clone().is_none() {
            continue;
        }
        let evm_owner = profile.owned_by.address.0.to_ascii_lowercase();
        // fetch default profile id for lens-v2
        let default_profile_id = get_default_profile_id(&evm_owner).await?;
        let t = save_profile(&cli, profile, default_profile_id).await?;
        if let Some(t) = t {
            next_targets.push(t);
        }
    }

    Ok(next_targets)
}

async fn get_default_profile_id(evm_address: &str) -> Result<Option<ProfileId>, Error> {
    // https://docs.lens.xyz/docs/default-profile
    // Even though default profiles are not part of the Lens V2 Protocol specification
    // the Lens API allows setting and fetching default profiles for any given EvmAddress.
    // If a Lens user has not explicitly set their default profile,
    // their oldest profile will be returned, prioritising profiles with linked handles.
    let default_operation = GetDefaultProfile::build(DefaultProfileVariables {
        evm_address: EvmAddress(evm_address.to_string()),
    });
    let default_response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(default_operation)
        .await;

    if default_response.is_err() {
        warn!(
            "Failed to fetch default profile.id: {}",
            default_response.unwrap_err(),
        );
        return Ok(None);
    }
    match default_response
        .unwrap()
        .data
        .map_or(None, |data| data.default_profile)
    {
        Some(profile) => Ok(Some(profile.id)),
        None => Ok(None),
    }
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
    // fetch default profile id for lens-v2
    let default_profile_id = get_default_profile_id(&owned_by_evm).await?;
    let mut next_targets: Vec<Target> = Vec::new();
    for profile in profiles.iter() {
        let t = save_profile(&cli, profile, default_profile_id.clone()).await?;
        if let Some(t) = t {
            next_targets.push(t);
        }
    }

    Ok(next_targets)
}

async fn save_profile(
    client: &Client<HttpConnector>,
    profile: &Profile,
    default_profile_id: Option<ProfileId>,
) -> Result<Option<Target>, Error> {
    if profile.handle.clone().is_none() {
        return Ok(None);
    }
    let mut is_default = false;
    if let Some(default_profile_id) = default_profile_id {
        if profile.id == default_profile_id {
            tracing::info!(
                "profile.id {:?} == default_profile_id {:?}",
                profile.id,
                default_profile_id
            );
            is_default = true;
        }
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
        reverse: Some(is_default),
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
        reverse: Some(is_default),
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
    if is_default {
        create_identity_domain_reverse_resolve_record(client, &addr, &lens, &reverse).await?;
    }
    Ok(Some(Target::Identity(Platform::Ethereum, owner.clone())))
}

#[async_trait]
impl DomainSearch for LensV2 {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        if name == "" {
            warn!("LensV2 handle_search(name='') is not a valid handle name");
            return Ok(vec![]);
        }
        debug!("LensV2 handle_search(name={})", name);

        let full_handle = format!("lens/{}", name);
        let profiles = query_by_handle(&full_handle).await?;
        if profiles.is_empty() {
            warn!("LensV2 handle_search(name={}) | No Result", name,);
            return Ok(vec![]);
        }
        let lens_profile = profiles.first().unwrap().clone();
        if lens_profile.handle.clone().is_none() {
            warn!("LensV2 handle_search(name={}) | lens handle is null", name,);
            return Ok(vec![]);
        }

        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            id: name.to_string(),
            updated_at: naive_now(),
        };

        let evm_owner = lens_profile.owned_by.address.0.to_ascii_lowercase();
        let handle_info = lens_profile.handle.clone().unwrap();
        let lens_handle = format!("{}.{}", handle_info.local_name, handle_info.namespace);
        let lens_display_name = lens_profile
            .metadata
            .clone()
            .map_or(None, |metadata| metadata.display_name);
        let created_at = utc_to_naive(lens_profile.created_at.clone().0)?;

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: evm_owner.clone(),
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
            uid: Some(lens_profile.id.clone().0.to_string()),
            created_at: Some(created_at),
            display_name: lens_display_name,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: Some("https://hey.xyz/u/".to_owned() + &handle_info.local_name),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Lens,
            transaction: Some(lens_profile.tx_hash.clone().0),
            id: lens_profile.id.clone().0.to_string(),
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

        let collection_edge = PartOfCollection {
            platform: Platform::Lens,
            name: lens_handle.clone(),
            tld: EXT::Lens.to_string(),
            status: DomainStatus::Taken,
        };

        let hd = hold.wrapper(&addr, &lens, HOLD_IDENTITY);
        let rs = resolve.wrapper(&lens, &addr, RESOLVE);
        let c = collection_edge.wrapper(&domain_collection, &lens, PART_OF_COLLECTION);

        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        edges.push(EdgeWrapperEnum::new_resolve(rs));
        edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));

        Ok(edges)
    }
}
