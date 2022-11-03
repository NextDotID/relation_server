#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::{
    config::C,
    error::Error,
    graph::{
        create_identity_to_contract_record,
        edge::{hold::Hold, resolve::DomainNameSystem, Resolve},
        new_db_connection,
        vertex::{
            contract::{Chain, ContractCategory},
            Contract, ContractRecord, Identity,
        },
        Edge, Vertex,
    },
    upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList},
    util::{naive_now, parse_timestamp},
};
use aragog::DatabaseConnection;
use async_trait::async_trait;
use gql_client::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Serialize, Debug)]
struct QueryVars {
    target: String,
}

#[derive(Deserialize, Debug)]
struct SimpleResponse {
    ping: String,
}

#[derive(Deserialize, Debug)]
struct ProfileQueryResponse {
    profiles: Vec<Profile>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[allow(dead_code)]
struct Profile {
    id: String,
    handle: String,
    owner: String,
    imageURI: String,
    createdOn: String,
}

const QUERY_LENS_PROFILE: &str = r#"
        query ProfileQuerrry($target: String!) {
            profiles(where: { handle: $target}) {
                id
                handle    
                owner
            }
        }
    "#;

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

        //perform_fetch(target).await
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
    let query: String;
    let target_var: String;
    // match target {
    //     Target::Identity(_platform_, identity) => {
    //         query = QUERY_BY_WALLET.to_string();
    //         target_var = identity.clone();
    //     }
    //     Target::NFT(_chain, _category, _contract_addr, ens_name) => {
    //         query = QUERY_BY_ENS.to_string();
    //         target_var = ens_name.clone();
    //     }
    // }

    // let mut header = HashMap::new();
    // header.insert("Content-Type", "application/json");
    // header.insert("Accept", "application/json");
    // header.insert("Origin", "https://api.lens.dev");

    let client = Client::new(&C.upstream.lens_api.url);
    println!("url: {}", &C.upstream.lens_api.url);
    query = QUERY_LENS_PROFILE.to_string();
    //query = QUERY_DEMO.to_string();
    target_var = target.identity()?;
    println!("query {}", query);
    println!("target_var {}", target_var);
    let vars = QueryVars { target: target_var };
    //println!("var {:?}", vars);

    let resp = client
        .query_with_vars::<ProfileQueryResponse, QueryVars>(&query, vars)
        .await;
    println!("{:?}", resp);

    // if resp.is_err() {
    //     warn!(
    //         "Lens Protocol API {} | Failed to fetch: {}",
    //         target,
    //         resp.unwrap_err(),
    //     );
    //     return Ok(vec![]);
    // }

    // let res = resp.unwrap().unwrap();
    // println!("res {:?}", res);

    //let db = new_db_connection().await?;
    let mut next_targets: TargetProcessedList = vec![];

    // for domain in res.domains.into_iter() {
    //     // Create own record
    //     let contract_record = create_or_update_own(&db, &domain).await?;

    //     // Deal with resolve target.
    //     let resolved_address = domain.resolvedAddress.map(|r| r.id);
    //     match resolved_address.clone() {
    //         Some(address) => {
    //             // Create resolve record
    //             debug!("TheGraph {} | Resolved address: {}", target, address);
    //             let resolve_target = Identity {
    //                 uuid: Some(Uuid::new_v4()),
    //                 platform: Platform::Ethereum,
    //                 identity: address.clone(),
    //                 created_at: None,
    //                 display_name: None,
    //                 added_at: naive_now(),
    //                 avatar_url: None,
    //                 profile_url: None,
    //                 updated_at: naive_now(),
    //             }
    //             .create_or_update(&db)
    //             .await?;
    //             let resolve = Resolve {
    //                 uuid: Uuid::new_v4(),
    //                 source: DataSource::TheGraph,
    //                 system: DomainNameSystem::ENS,
    //                 name: domain.name.clone(),
    //                 fetcher: DataFetcher::RelationService,
    //                 updated_at: naive_now(),
    //             };

    //             resolve
    //                 .connect(&db, &contract_record, &resolve_target)
    //                 .await?;
    //         }
    //         None => {
    //             // Resolve record not existed anymore. Maybe deleted by user.
    //             // TODO: Should find existed connection and delete it.
    //         }
    //     }

    // Append up_next
    // match target {
    //     Target::Identity(_, _) => next_targets.push(Target::NFT(
    //         Chain::Ethereum,
    //         ContractCategory::ENS,
    //         ContractCategory::ENS.default_contract_address().unwrap(),
    //         domain.name.clone(),
    //     )),
    //     Target::NFT(_, _, _, _) => {
    //         let owner_address = domain.owner.id.clone();
    //         next_targets.push(Target::Identity(Platform::Ethereum, owner_address.clone()));
    //         if resolved_address.is_some() && resolved_address != Some(owner_address) {
    //             next_targets.push(Target::Identity(
    //                 Platform::Ethereum,
    //                 resolved_address.unwrap(),
    //             ));
    //         }
    //     }
    // }
    //}
    Ok(next_targets)
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
