mod tests;

use crate::{
    config::C,
    error::Error,
    graph::{
        create_identity_to_contract_record,
        edge::hold::Hold,
        new_db_connection,
        vertex::{
            contract::{Chain, ContractCategory},
            Contract, Identity,
        },
    },
    upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList},
    util::{naive_now, parse_timestamp},
};
use async_trait::async_trait;
use gql_client::Client;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct QueryVars {
    target: String,
}

#[derive(Deserialize, Debug)]
pub struct QueryResponse {
    domains: Vec<Domain>,
    // transfers: Option<Vec<EthQueryResponseTransfers>>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Domain {
    /// ENS name (`something.eth`)
    name: String,
    /// Creation timestamp (in secods)
    createdAt: String,
    /// ETH event logs for this ENS.
    events: Vec<DomainEvent>,
    /// Reverse resolve record setted on this ENS.
    resolvedAddress: Option<Account>,
    /// Owner info
    owner: Account,
}

#[derive(Deserialize, Debug)]
pub struct Account {
    /// Ethereum wallet
    id: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct DomainEvent {
    pub blockNumber: u128,
    pub transactionID: String,
    pub domain: DomainTiny,
}

#[derive(Deserialize, Debug)]
pub struct DomainTiny {
    name: String,
}

const QUERY_BY_ENS: &str = r#"
        query OwnerAddressByENS($target: String!){
            domains(where: { name: $target }) {
                name
                createdAt
                events {
                    blockNumber
                    transactionID
                    domain {
                        name
                    }
                }
                resolvedAddress {
                  id
                }
                owner{
                  id
                }
              }
        }
    "#;

const QUERY_BY_WALLET: &str = r#"
        query ENSByOwnerAddress($target: String!){
            domains(where: { owner: $target }) {
                name
                createdAt
                events {
                    blockNumber
                    transactionID
                    domain {
                        name
                    }
                }
                resolvedAddress {
                  id
                }
                owner {
                  id
                }
              }
        }
    "#;

pub struct TheGraph {}

#[async_trait]
impl Fetcher for TheGraph {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        perform_fetch(target).await
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
            || target.in_nft_supported(vec![ContractCategory::ENS], vec![Chain::Ethereum])
    }
}

async fn perform_fetch(target: &Target) -> Result<TargetProcessedList, Error> {
    let query: String;
    let target_var: String;
    match target {
        Target::Identity(_platform_, identity) => {
            query = QUERY_BY_WALLET.to_string();
            target_var = identity.clone();
        }
        Target::NFT(_chain, _category, _contract_addr, ens_name) => {
            query = QUERY_BY_ENS.to_string();
            target_var = ens_name.clone();
        }
    }

    let client = Client::new(&C.upstream.the_graph.ens);
    let vars = QueryVars { target: target_var };

    let resp = client
        .query_with_vars::<QueryResponse, QueryVars>(&query, vars)
        .await;

    if resp.is_err() {
        warn!(
            "TheGraph {} | Failed to fetch: {}",
            target,
            resp.unwrap_err(),
        );
        return Ok(vec![]);
    }

    let res = resp.unwrap().unwrap();
    if res.domains.is_empty() {
        info!("TheGraph {} | No result", target);
        return Ok(vec![]);
    }
    let db = new_db_connection().await?;
    let mut next_targets: TargetProcessedList = vec![];

    for domain in res.domains.iter() {
        let creation_tx = domain
            .events
            .first() // TODO: really?
            .map(|event| event.transactionID.clone());
        let resolved_address = domain.resolvedAddress.as_ref().map(|r| r.id.clone());
        let ens_created_at = parse_timestamp(&domain.createdAt).ok();
        let from: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: domain.owner.id.clone(),
            created_at: None,
            display_name: resolved_address.unwrap_or("".into()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
        };
        let to: Contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::ENS,
            address: ContractCategory::ENS.default_contract_address().unwrap(),
            chain: Chain::Ethereum,
            symbol: None,
            updated_at: naive_now(),
        };
        let ownership: Hold = Hold {
            uuid: Uuid::new_v4(),
            transaction: creation_tx,
            id: domain.name.clone(),
            source: DataSource::TheGraph,
            created_at: ens_created_at,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
        };
        create_identity_to_contract_record(&db, &from, &to, &ownership).await?;

        // Push up_next record
        match target {
            Target::Identity(_, _) => next_targets.push(Target::NFT(
                Chain::Ethereum,
                ContractCategory::ENS,
                ContractCategory::ENS.default_contract_address().unwrap(),
                domain.name.clone(),
            )),
            Target::NFT(_, _, _, _) => {
                next_targets.push(Target::Identity(Platform::Ethereum, from.identity))
            }
        }
    }
    Ok(next_targets)
}
