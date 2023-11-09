#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::create_identity_domain_resolve_record;
use crate::tigergraph::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::create_identity_to_identity_hold_record;
use crate::tigergraph::edge::{Hold, Resolve};
use crate::tigergraph::vertex::Identity;
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{make_http_client, naive_now};
use async_trait::async_trait;
use cynic::{http::SurfExt, QueryBuilder};
use hyper::{client::HttpConnector, Client};
use tracing::{info, warn};
use uuid::Uuid;

use self::queries::Profile;

#[cynic::schema_for_derives(file = "src/upstream/lens/schema.graphql", module = "schema")]
mod queries {
    use super::schema;

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", argument_struct = "ProfileQueryArguments")]
    pub struct ProfileQuery {
        #[arguments(request = SingleProfileQueryRequest { handle: args.request.handle.clone() })]
        pub profile: Option<Profile>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Profile {
        pub bio: Option<String>,
        pub handle: String,
        pub id: String,
        pub is_default: bool,
        pub is_followed_by_me: bool,
        pub name: Option<String>,
        pub metadata: Option<String>,
        pub owned_by: String,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct ProfileQueryArguments {
        pub request: SingleProfileQueryRequest,
    }

    #[derive(cynic::InputObject, Debug)]
    pub struct SingleProfileQueryRequest {
        pub handle: Option<String>,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct ProfilesQueryArguments {
        pub request: ProfileQueryRequest,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", argument_struct = "ProfilesQueryArguments")]
    pub struct ProfilesQuery {
        #[arguments(request = ProfileQueryRequest {owned_by:args.request.owned_by.clone() })]
        pub profiles: PaginatedProfileResult,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct PaginatedProfileResult {
        pub items: Vec<Profile>,
    }

    #[derive(cynic::InputObject, Debug)]
    pub struct ProfileQueryRequest {
        pub owned_by: Option<Vec<String>>,
    }

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct EthereumAddress(pub String);
    cynic::impl_scalar!(String, schema::EthereumAddress);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct Handle(pub String);
    cynic::impl_scalar!(String, schema::Handle);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct ProfileId(pub String);
    cynic::impl_scalar!(String, schema::ProfileId);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct Url(pub String);
    cynic::impl_scalar!(String, schema::Url);

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct LimitScalar(pub String);
    cynic::impl_scalar!(String, schema::LimitScalar);
}

mod schema {
    cynic::use_schema!("src/upstream/lens/schema.graphql");
}

pub struct Lens {}

#[async_trait]
impl Fetcher for Lens {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target.platform()? {
            Platform::Ethereum => fetch_by_addr(target).await,
            Platform::Lens => fetch_by_lens_profile(target).await,
            _ => Ok(vec![]),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Lens])
    }
}

/// https://docs.lens.xyz/docs/get-profiles
async fn fetch_by_addr(target: &Target) -> Result<TargetProcessedList, Error> {
    use queries::*;

    let operation = ProfilesQuery::build(ProfilesQueryArguments {
        request: ProfileQueryRequest {
            owned_by: Some(vec![target.identity()?]),
        },
    });

    let response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(operation)
        .await;

    if response.is_err() {
        warn!(
            "Lens target {} | Failed to fetch: {}",
            target,
            response.unwrap_err(),
        );
        return Ok(vec![]);
    }
    let data = response.unwrap().data.unwrap().profiles.items;
    if data.len() == 0 {
        info!("Lens profile {} | No result", target);
        return Ok(vec![]);
    }
    let cli = make_http_client();
    for profile in data.into_iter() {
        save_profile(&cli, &profile).await?;
    }
    // there is no other upstream can get lens protocol
    Ok(vec![])
}

async fn fetch_by_lens_profile(target: &Target) -> Result<TargetProcessedList, Error> {
    use queries::*;

    let operation = ProfileQuery::build(ProfileQueryArguments {
        request: SingleProfileQueryRequest {
            handle: Some(target.identity()?),
        },
    });

    let response = surf::post(C.upstream.lens_api.url.clone())
        .run_graphql(operation)
        .await;

    if response.is_err() {
        warn!(
            "Lens target {} | Failed to fetch: {}",
            target,
            response.unwrap_err(),
        );
        return Ok(vec![]);
    }

    let data: Option<Profile> = response.unwrap().data.unwrap().profile;
    if data.is_none() {
        info!("Lens profile {} | No result", target);
        return Ok(vec![]);
    }
    let profile: Profile = data.unwrap();
    let cli = make_http_client();
    save_profile(&cli, &profile).await?;

    Ok(vec![Target::Identity(
        Platform::Ethereum,
        profile.owned_by.to_lowercase(),
    )])
}

async fn save_profile(client: &Client<HttpConnector>, profile: &Profile) -> Result<(), Error> {
    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: profile.owned_by.clone().to_lowercase(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Lens,
        identity: profile.handle.clone(),
        uid: Some(profile.id.clone()),
        created_at: None,
        display_name: profile.name.clone(),
        added_at: naive_now(),
        avatar_url: profile.metadata.clone(),
        profile_url: Some("https://lenster.xyz/u/".to_owned() + &profile.handle.clone()),
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        transaction: None,
        id: profile.id.clone(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        system: DomainNameSystem::Lens,
        name: profile.handle.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };
    create_identity_to_identity_hold_record(client, &from, &to, &hold).await?;
    create_identity_domain_resolve_record(client, &from, &to, &resolve).await?;

    if profile.is_default {
        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Lens,
            system: DomainNameSystem::Lens,
            name: profile.handle.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        create_identity_domain_reverse_resolve_record(client, &to, &from, &resolve).await?;
    }
    Ok(())
}
