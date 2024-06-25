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
use gql_client::Client as GQLClient;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

#[derive(Serialize)]
struct QueryVars {
    target: String,
}

#[derive(Deserialize, Debug)]
struct QueryResponse {
    domains: Vec<Domain>,
    #[serde(rename = "wrappedDomains")]
    wrapped_domains: Vec<WrappedDomain>,
}

#[derive(Deserialize, Debug, Clone)]
struct Domain {
    /// ENS name (`something.eth`)
    name: String,
    /// Creation timestamp (in secods)
    #[serde(rename = "createdAt")]
    created_at: String,
    registration: Option<Registration>,
    /// ETH event logs for this ENS.
    events: Vec<DomainEvent>,
    /// Reverse resolve record set on this ENS.
    #[serde(rename = "resolvedAddress")]
    resolved_address: Option<Account>,
    /// Owner info
    owner: Account,
}

#[derive(Deserialize, Debug, Clone)]
struct Registration {
    #[allow(dead_code)]
    #[serde(rename = "registrationDate")]
    registration_date: String,
    #[serde(rename = "expiryDate")]
    expiry_date: String,
}

#[derive(Deserialize, Debug, Clone)]
struct WrappedDomain {
    name: String,
    owner: Account,
    domain: Domain,
}

#[derive(Deserialize, Debug, Clone)]
struct Account {
    /// Ethereum wallet
    id: String,
}

#[derive(Deserialize, Debug, Clone)]
struct DomainEvent {
    #[serde(rename = "transactionID")]
    transaction_id: String,
}

fn choose_endpoint() -> String {
    let mut options: Vec<String> = Vec::new();

    if let Some(value) = C.upstream.the_graph.subgraph0.clone() {
        options.push(value);
    }
    if let Some(value) = C.upstream.the_graph.subgraph1.clone() {
        options.push(value);
    }
    if let Some(value) = C.upstream.the_graph.subgraph2.clone() {
        options.push(value);
    }
    if let Some(value) = C.upstream.the_graph.subgraph3.clone() {
        options.push(value);
    }
    if let Some(value) = C.upstream.the_graph.subgraph4.clone() {
        options.push(value);
    }

    if options.is_empty() {
        C.upstream.the_graph.ens.clone()
    } else {
        let mut rng = thread_rng();
        options.choose(&mut rng).cloned().unwrap().clone()
    }
}

const QUERY_BY_ENS: &str = r#"
        query OwnerAddressByENS($target: String!){
            domains(where: { name: $target }) {
                name
                createdAt
                registration {
                    registrationDate
                    expiryDate
                }
                events(first: 1) {
                    transactionID
                }
                resolvedAddress {
                  id
                }
                owner{
                  id
                }
              }
            wrappedDomains(where: { name: $target }) {
              name
              domain {
                name
                createdAt
                registration {
                    registrationDate
                    expiryDate
                }
                events(first: 1) {
                    transactionID
                }
                resolvedAddress {
                  id
                }
                owner{
                  id
                }
              }
              owner {
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
                registration {
                    registrationDate
                    expiryDate
                }
                events(first: 1) {
                    transactionID
                }
                resolvedAddress {
                  id
                }
                owner {
                  id
                }
              }
            wrappedDomains(where: { owner: $target }) {
              name
              domain {
                name
                createdAt
                registration {
                    registrationDate
                    expiryDate
                }
                events(first: 1) {
                    transactionID
                }
                resolvedAddress {
                  id
                }
                owner{
                  id
                }
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

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        batch_perform_fetch(target).await
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
            || target.in_nft_supported(vec![ContractCategory::ENS], vec![Chain::Ethereum])
    }
}

/// reverse lookup for ENS is not provided by official TheGraph for now.
/// See also: https://github.com/ensdomains/ens-subgraph/issues/25
/// Consider deploy a self-hosted reverse lookup service like:
/// https://github.com/fafrd/ens-reverse-lookup
async fn fetch_domains(target: &Target) -> Result<Vec<Domain>, Error> {
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
    let endpoints = choose_endpoint();
    let client = GQLClient::new(&endpoints);
    let vars = QueryVars { target: target_var };

    let resp = client.query_with_vars::<QueryResponse, QueryVars>(&query, vars);

    let data: Option<QueryResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), resp).await {
            Ok(resp) => match resp {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(?target, ?err, "TheGraph: Failed to fetch");
                    None
                }
            },
            Err(_) => {
                warn!(?target, "TheGraph: Timeout: no response in 5 seconds.");
                None
            }
        };

    if data.is_none() {
        info!(?target, "TheGraph: No result");
        return Ok(vec![]);
    }
    let res = data.unwrap();
    debug!(
        ?target,
        wrapped = res.wrapped_domains.len(),
        domains = res.domains.len(),
        "Records found."
    );
    let mut merged_domains: Vec<Domain> = vec![];
    // Rewrite correct owner info for wrapped domains.
    for wd in res.wrapped_domains.into_iter() {
        debug!(?target, domain = wd.name, "TheGraph: Wrapped ENS found.");
        let mut domain = wd.domain.clone();
        domain.owner = wd.owner;
        merged_domains.push(domain);
    }
    for domain in res.domains.into_iter() {
        if merged_domains.iter().any(|md| md.name == domain.name) {
            debug!(
                ?target,
                domain = domain.name,
                "TheGraph: Wrapped ENS found before. Skip this."
            );
            continue;
        } else {
            merged_domains.push(domain);
        }
    }
    Ok(merged_domains)
}

async fn batch_perform_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let merged_domains = fetch_domains(target).await?;
    if merged_domains.is_empty() {
        info!(?target, "TheGraph: No result");
        return Ok((vec![], vec![]));
    }

    let platform = match target {
        Target::Identity(_platform, _identity) => *_platform,
        Target::NFT(_chain, _category, _contract_addr, _ens_name) => Platform::ENS,
    };

    let mut next_targets: TargetProcessedList = vec![];
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    for domain in merged_domains.into_iter() {
        let creation_tx = domain
            .events
            .first()
            .map(|event| event.transaction_id.clone());
        let ens_created_at = parse_timestamp(&domain.created_at).ok();
        let ens_expired_at = match &domain.registration {
            Some(registration) => parse_timestamp(&registration.expiry_date).ok(),
            None => None,
        };

        let owner = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: domain.owner.id.clone(),
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
        let ens_domain: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::ENS,
            identity: domain.name.clone(),
            uid: None,
            created_at: ens_created_at,
            display_name: Some(domain.name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: ens_expired_at,
            reverse: Some(false),
        };
        let contract = Contract {
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
            expired_at: ens_expired_at,
        };

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &owner, HYPER_EDGE),
        ));

        let resolved_address = domain.resolved_address.map(|r| r.id);
        match resolved_address.clone() {
            None => {
                // Resolve record not existed anymore. Save owner address.
                trace!(
                    ?target,
                    "TheGraph: Resolve address not existed. Save owner address"
                );
                // hold record
                let hd = ownership.wrapper(&owner, &ens_domain, HOLD_IDENTITY);
                let hdc = ownership.wrapper(&owner, &contract, HOLD_CONTRACT);
                edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
                // Append up_next
                if platform == Platform::ENS {
                    next_targets.push(Target::Identity(
                        Platform::Ethereum,
                        domain.owner.id.clone(),
                    ));
                }
            }
            Some(address) => {
                // Filter zero address (without last 4 digits)
                if !address.starts_with("0x000000000000000000000000000000000000") {
                    // Create resolve record
                    debug!(?target, address, domain = domain.name, "TheGraph: Resolved");
                    let resolve = Resolve {
                        uuid: Uuid::new_v4(),
                        source: DataSource::TheGraph,
                        system: DomainNameSystem::ENS,
                        name: domain.name.clone(),
                        fetcher: DataFetcher::RelationService,
                        updated_at: naive_now(),
                    };

                    let domain_owner = domain.owner.id.clone();

                    if address == domain_owner {
                        // ens_domain will be added to hyper_vertex IdentitiesGraph
                        // only when resolvedAddress == owner
                        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
                            &hv,
                            &ens_domain,
                            HYPER_EDGE,
                        )));

                        // hold record
                        let hd = ownership.wrapper(&owner, &ens_domain, HOLD_IDENTITY);
                        let hdc = ownership.wrapper(&owner, &contract, HOLD_CONTRACT);
                        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                        edges.push(EdgeWrapperEnum::new_hold_contract(hdc));

                        // resolve record
                        let rs = resolve.wrapper(&ens_domain, &owner, RESOLVE);
                        let rsc = resolve.wrapper(&contract, &owner, RESOLVE_CONTRACT);
                        edges.push(EdgeWrapperEnum::new_resolve(rs));
                        edges.push(EdgeWrapperEnum::new_resolve_contract(rsc));

                        // Append up_next
                        if platform == Platform::ENS {
                            next_targets
                                .push(Target::Identity(Platform::Ethereum, domain_owner.clone()));
                        }
                    } else {
                        debug!(
                            ?target,
                            address, domain_owner, "TheGraph: address != domain_owner"
                        );
                        // hold record
                        let hd = ownership.wrapper(&owner, &ens_domain, HOLD_IDENTITY);
                        let hdc = ownership.wrapper(&owner, &contract, HOLD_CONTRACT);
                        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                        edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
                        // Append up_next
                        if platform == Platform::ENS {
                            next_targets
                                .push(Target::Identity(Platform::Ethereum, domain_owner.clone()));
                        }
                    }
                }
            }
        }
    }
    next_targets.dedup();
    return Ok((next_targets, edges));
}

/// Focus on `ENS Domains` Saveing.
async fn perform_fetch(target: &Target) -> Result<TargetProcessedList, Error> {
    let merged_domains = fetch_domains(target).await?;
    if merged_domains.is_empty() {
        info!(?target, "TheGraph: No result");
        return Ok(vec![]);
    }

    let cli = make_http_client();
    let mut next_targets: TargetProcessedList = vec![];

    for domain in merged_domains.into_iter() {
        let creation_tx = domain
            .events
            .first()
            .map(|event| event.transaction_id.clone());
        let ens_created_at = parse_timestamp(&domain.created_at).ok();
        let ens_expired_at = match &domain.registration {
            Some(registration) => parse_timestamp(&registration.expiry_date).ok(),
            None => None,
        };

        let owner = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: domain.owner.id.clone(),
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
        let ens_domain: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::ENS,
            identity: domain.name.clone(),
            uid: None,
            created_at: ens_created_at,
            display_name: Some(domain.name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: ens_expired_at,
            reverse: Some(false),
        };
        let contract = Contract {
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
            expired_at: ens_expired_at,
        };

        let resolved_address = domain.resolved_address.map(|r| r.id);
        match resolved_address.clone() {
            None => {
                // Resolve record not existed anymore. Maybe deleted by user.
                trace!(?target, "TheGraph: Resolve address not existed.");
                // Save owner address
                create_ens_identity_ownership(&cli, &owner, &ens_domain, &ownership).await?;
                create_identity_to_contract_hold_record(&cli, &owner, &contract, &ownership)
                    .await?;
            }
            Some(address) => {
                // Filter zero address (without last 4 digits)
                if !address.starts_with("0x000000000000000000000000000000000000") {
                    // Create resolve record
                    debug!(?target, address, domain = domain.name, "TheGraph: Resolved");
                    let resolve_target = Identity {
                        uuid: Some(Uuid::new_v4()),
                        platform: Platform::Ethereum,
                        identity: address.clone(),
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

                    let resolve = Resolve {
                        uuid: Uuid::new_v4(),
                        source: DataSource::TheGraph,
                        system: DomainNameSystem::ENS,
                        name: domain.name.clone(),
                        fetcher: DataFetcher::RelationService,
                        updated_at: naive_now(),
                    };

                    let domain_owner = domain.owner.id.clone();
                    if address != domain_owner {
                        // Save hold, resolve but not connect IdentityGraph HyperVetex
                        // create owner-hold-ens(contract)
                        create_ens_identity_ownership(&cli, &owner, &ens_domain, &ownership)
                            .await?;
                        create_identity_to_contract_hold_record(
                            &cli, &owner, &contract, &ownership,
                        )
                        .await?;

                        // create ens-resolve-address(contract)
                        create_ens_identity_resolve(&cli, &ens_domain, &resolve_target, &resolve)
                            .await?;
                        create_contract_to_identity_resolve_record(
                            &cli,
                            &contract,
                            &resolve_target,
                            &resolve,
                        )
                        .await?;
                    } else {
                        // Save ens_identity as Identity in IdentityGraph
                        // create owner-hold-ens(contract)
                        create_ens_identity_ownership(&cli, &owner, &ens_domain, &ownership)
                            .await?;
                        create_identity_to_contract_hold_record(
                            &cli, &owner, &contract, &ownership,
                        )
                        .await?;

                        // create ens-resolve-addres(IdentityGraph)
                        create_identity_domain_resolve_record(
                            &cli,
                            &ens_domain,
                            &resolve_target,
                            &resolve,
                        )
                        .await?;
                        create_contract_to_identity_resolve_record(
                            &cli,
                            &contract,
                            &resolve_target,
                            &resolve,
                        )
                        .await?;
                    }
                }
            }
        }

        // Append up_next
        match target {
            Target::Identity(_, _) => next_targets.push(Target::NFT(
                Chain::Ethereum,
                ContractCategory::ENS,
                ContractCategory::ENS.default_contract_address().unwrap(),
                domain.name.clone(),
            )),
            Target::NFT(_, _, _, _) => {
                let owner_address = domain.owner.id.clone();
                next_targets.push(Target::Identity(Platform::Ethereum, owner_address.clone()));
                if resolved_address.is_some() && resolved_address != Some(owner_address) {
                    next_targets.push(Target::Identity(
                        Platform::Ethereum,
                        resolved_address.unwrap(),
                    ));
                }
            }
        }
    }

    Ok(next_targets)
}
