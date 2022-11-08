#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::{
    config::C,
    error::Error,
    graph::{
        create_identity_to_contract_record, create_identity_to_identity_hold_record,
        edge::{hold::Hold, resolve::DomainNameSystem, Resolve},
        new_db_connection,
        vertex::{
            contract::{Chain, ContractCategory},
            Contract, ContractRecord, Identity,
        },
        Edge, Vertex,
    },
    upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList},
    util::naive_now,
};
use aragog::DatabaseConnection;
use async_trait::async_trait;
use cynic::{http::SurfExt, QueryBuilder};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

// #[derive(Serialize, Debug)]
// struct QueryVars {
//     target: String,
// }

// #[derive(Deserialize, Debug)]
// struct ProfileQueryResponse {
//     profiles: Vec<Profile>,
// }

// #[derive(Deserialize, Debug)]
// #[allow(non_snake_case)]
// #[allow(dead_code)]
// struct Profile {
//     id: String,
//     name: String,
//     bio: String,
//     //imageURI: String,
//     ownerBy: String,
//     isDefault: bool,
// }

#[cynic::schema_for_derives(file = "schema.graphql", module = "schema")]
mod queries {
    use super::schema;

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct ProfileQueryArguments {
        pub request: SingleProfileQueryRequest,
    }

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

    #[derive(cynic::InputObject, Debug)]
    pub struct SingleProfileQueryRequest {
        pub handle: Option<String>,
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
}

mod schema {
    cynic::use_schema!("schema.graphql");
}

pub struct Lens {}

#[async_trait]
impl Fetcher for Lens {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target.platform()? {
            Platform::Ethereum => todo!(),
            Platform::Lens => fetch_by_lens_profile(target).await,
            _ => Ok(vec![]),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Lens])
    }
}

/// https://docs.lens.xyz/docs/get-profiles
// async fn fetch_by_addr(target: &Target) -> Result<TargetProcessedList, Error> {
//     let query: String;
//     let target_var: String;
//     match target {
//         Target::Identity(_platform_, identity) => {
//             query = QUERY_BY_WALLET.to_string();
//             target_var = identity.clone();
//         }
//         Target::NFT(_chain, _category, _contract_addr, ens_name) => {
//             query = QUERY_BY_ENS.to_string();
//             target_var = ens_name.clone();
//         }
//     }

//     let client = Client::new(&C.upstream.the_graph.ens);
//     let vars = QueryVars { target: target_var };

//     let resp = client
//         .query_with_vars::<QueryResponse, QueryVars>(&query, vars)
//         .await;

//     if resp.is_err() {
//         warn!(
//             "TheGraph {} | Failed to fetch: {}",
//             target,
//             resp.unwrap_err(),
//         );
//         return Ok(vec![]);
//     }

//     let res = resp.unwrap().unwrap();
//     if res.domains.is_empty() {
//         info!("TheGraph {} | No result", target);
//         return Ok(vec![]);
//     }
//     let db = new_db_connection().await?;
//     let mut next_targets: TargetProcessedList = vec![];

//     for domain in res.domains.into_iter() {
//         // Create own record
//         let contract_record = create_or_update_own(&db, &domain).await?;

//         // Deal with resolve target.
//         let resolved_address = domain.resolvedAddress.map(|r| r.id);
//         match resolved_address.clone() {
//             Some(address) => {
//                 // Create resolve record
//                 debug!("TheGraph {} | Resolved address: {}", target, address);
//                 let resolve_target = Identity {
//                     uuid: Some(Uuid::new_v4()),
//                     platform: Platform::Ethereum,
//                     identity: address.clone(),
//                     created_at: None,
//                     display_name: None,
//                     added_at: naive_now(),
//                     avatar_url: None,
//                     profile_url: None,
//                     updated_at: naive_now(),
//                 }
//                 .create_or_update(&db)
//                 .await?;
//                 let resolve = Resolve {
//                     uuid: Uuid::new_v4(),
//                     source: DataSource::TheGraph,
//                     system: DomainNameSystem::ENS,
//                     name: domain.name.clone(),
//                     fetcher: DataFetcher::RelationService,
//                     updated_at: naive_now(),
//                 };

//                 resolve
//                     .connect(&db, &contract_record, &resolve_target)
//                     .await?;
//             }
//             None => {
//                 // Resolve record not existed anymore. Maybe deleted by user.
//                 // TODO: Should find existed connection and delete it.
//             }
//         }

//         // Append up_next
//         match target {
//             Target::Identity(_, _) => next_targets.push(Target::NFT(
//                 Chain::Ethereum,
//                 ContractCategory::ENS,
//                 ContractCategory::ENS.default_contract_address().unwrap(),
//                 domain.name.clone(),
//             )),
//             Target::NFT(_, _, _, _) => {
//                 let owner_address = domain.owner.id.clone();
//                 next_targets.push(Target::Identity(Platform::Ethereum, owner_address.clone()));
//                 if resolved_address.is_some() && resolved_address != Some(owner_address) {
//                     next_targets.push(Target::Identity(
//                         Platform::Ethereum,
//                         resolved_address.unwrap(),
//                     ));
//                 }
//             }
//         }
//     }
//     Ok(next_targets)
// }

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
    let db = new_db_connection().await?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: profile.owned_by.clone().to_lowercase(),
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
        identity: target.identity()?,
        created_at: None,
        display_name: profile.name,
        added_at: naive_now(),
        avatar_url: profile.metadata,
        profile_url: Some("https://lenster.xyz/u/".to_owned() + &target.identity()?),
        updated_at: naive_now(),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Lens,
        transaction: None,
        id: profile.id,
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
    };
    let from_record = from.create_or_update(&db).await?;
    let to_record = to.create_or_update(&db).await?;
    hold.connect(&db, &from_record, &to_record).await?;
    //create_identity_to_identity_hold_record(&db, &from, &to, &hold).await;

    if profile.is_default {
        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Lens,
            system: DomainNameSystem::Lens,
            name: target.identity()?,
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        resolve.connect(&db, &to_record, &from_record).await?;
    }

    Ok(vec![Target::Identity(
        Platform::Ethereum,
        profile.owned_by.to_lowercase(),
    )])
}

// async fn create_or_update_own(
//     db: &DatabaseConnection,
//     domain: &Domain,
// ) -> Result<ContractRecord, Error> {
//     let creation_tx = domain
//         .events
//         .first() // TODO: really?
//         .map(|event| event.transactionID.clone());
//     let ens_created_at = parse_timestamp(&domain.createdAt).ok();
//     let owner = Identity {
//         uuid: Some(Uuid::new_v4()),
//         platform: Platform::Ethereum,
//         identity: domain.owner.id.clone(),
//         created_at: None,
//         display_name: None,
//         added_at: naive_now(),
//         avatar_url: None,
//         profile_url: None,
//         updated_at: naive_now(),
//     };
//     let conrtract = Contract {
//         uuid: Uuid::new_v4(),
//         category: ContractCategory::ENS,
//         address: ContractCategory::ENS.default_contract_address().unwrap(),
//         chain: Chain::Ethereum,
//         symbol: None,
//         updated_at: naive_now(),
//     };
//     let ownership: Hold = Hold {
//         uuid: Uuid::new_v4(),
//         transaction: creation_tx,
//         id: domain.name.clone(),
//         source: DataSource::TheGraph,
//         created_at: ens_created_at,
//         updated_at: naive_now(),
//         fetcher: DataFetcher::RelationService,
//     };
//     let (_owner_record, contract_record, _hold_record) =
//         create_identity_to_contract_record(db, &owner, &conrtract, &ownership).await?;
//     Ok(contract_record)
// }
