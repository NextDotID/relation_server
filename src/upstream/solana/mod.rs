mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, HyperEdge, PartOfCollection, Proof, Resolve, Wrapper, HOLD_CONTRACT, HOLD_IDENTITY,
    HYPER_EDGE, PART_OF_COLLECTION, PROOF_EDGE, PROOF_REVERSE_EDGE, RESOLVE, REVERSE_RESOLVE,
};
use crate::tigergraph::upsert::create_ens_identity_ownership;
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_hold_record;
use crate::tigergraph::upsert::create_identity_to_identity_proof_two_way_binding;
use crate::tigergraph::upsert::create_isolated_vertex;
use crate::tigergraph::vertex::{Contract, DomainCollection, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::ProofLevel;
use crate::upstream::{Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, EXT};
use crate::util::{make_http_client, naive_now};
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::str::FromStr;
use tracing::{debug, trace, warn};
use uuid::Uuid;

/// Solana Sdk
use {
    sns_sdk::{
        derivation::get_hashed_name,
        non_blocking::resolve::{
            get_domains_owner, get_favourite_domain, resolve_name_registry, resolve_owner,
            resolve_reverse, resolve_reverse_batch,
        },
        record::{record_v2::deserialize_record_v2_content, Record},
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_program::pubkey::Pubkey,
    spl_name_service::state::get_seeds_and_key,
};

use super::{DomainSearch, Fetcher, Platform, Target, TargetProcessedList};

pub struct Solana {}

#[async_trait]
impl Fetcher for Solana {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        match target.platform()? {
            Platform::Solana => fetch_by_wallet(target).await,
            Platform::SNS => fetch_by_sns_handle(target).await,
            Platform::Twitter => fetch_by_twitter_handle(target).await,
            _ => Ok(vec![]),
        }
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        match target.platform()? {
            Platform::Solana => batch_fetch_by_wallet(target).await,
            Platform::SNS => batch_fetch_by_sns_handle(target).await,
            Platform::Twitter => batch_fetch_by_twitter_handle(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Solana, Platform::SNS, Platform::Twitter])
    }
}

async fn batch_fetch_by_wallet(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let owner: String = target.identity()?;
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let verified_owner = Pubkey::from_str(&owner)?;
    let resolve_domains = fetch_resolve_domains(&rpc_client, &owner).await?;

    let mut next_targets = TargetProcessedList::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    let mut solana = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Solana,
        identity: verified_owner.to_string(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: Some(format!(
            "https://www.sns.id/profile?pubkey={}&subView=Show+All",
            owner
        )),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    edges.push(EdgeWrapperEnum::new_hyper_edge(
        HyperEdge {}.wrapper(&hv, &solana, HYPER_EDGE),
    ));

    if resolve_domains.is_empty() {
        trace!(?target, "Solana resolve domains is null");
        return Ok((vec![], edges));
    }

    let favourite_domain = fetch_register_favourite(&rpc_client, &owner).await?;
    match favourite_domain {
        Some(favourite_domain) => {
            let format_sol = format_domain(&favourite_domain);
            trace!(?target, "Favourite Domain Founded({})", format_sol);
            solana.reverse = Some(true); // set reverse
            solana.display_name = Some(format_sol.clone());
            let farvourite_sns = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::SNS,
                identity: format_sol.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_sol.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/domain?domain={}",
                    favourite_domain
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(true),
            };

            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                system: DomainNameSystem::SNS,
                name: format_sol.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };
            edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
                &hv,
                &farvourite_sns,
                HYPER_EDGE,
            )));
            let rr = reverse.wrapper(&solana, &farvourite_sns, REVERSE_RESOLVE);
            edges.push(EdgeWrapperEnum::new_reverse_resolve(rr));
        }
        None => trace!(?target, "Favourite Domain Not Set"),
    };

    let twitter_handle = get_handle_and_registry_key(&rpc_client, &owner).await?;
    match twitter_handle {
        Some(twitter_handle) => {
            let format_twitter = twitter_handle.to_lowercase();
            let twitter = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Twitter,
                identity: format_twitter.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_twitter.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let proof_forward: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            let proof_backward: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &twitter, HYPER_EDGE),
            ));
            let pf = proof_forward.wrapper(&solana, &twitter, PROOF_EDGE);
            let pb = proof_backward.wrapper(&twitter, &solana, PROOF_REVERSE_EDGE);

            edges.push(EdgeWrapperEnum::new_proof_forward(pf));
            edges.push(EdgeWrapperEnum::new_proof_backward(pb));

            next_targets.push(Target::Identity(Platform::Twitter, format_twitter.clone()))
        }
        None => trace!(?target, "Twitter Record Not Set"),
    }

    trace!(
        ?target,
        domains = resolve_domains.len(),
        "Solana Resolve Domains"
    );
    for domain in resolve_domains.iter() {
        let format_sol_handle = format_domain(domain);
        let sns = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::SNS,
            identity: format_sol_handle.clone(),
            uid: None,
            created_at: None,
            display_name: Some(format_sol_handle.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: Some(format!("https://www.sns.id/domain?domain={}", domain)),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Solana,
            transaction: Some("".to_string()),
            id: format_sol_handle.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
            expired_at: None,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Solana,
            system: DomainNameSystem::SNS,
            name: format_sol_handle.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::SNS,
            address: ContractCategory::SNS.default_contract_address().unwrap(),
            chain: Chain::Solana,
            symbol: Some("SNS".to_string()),
            updated_at: naive_now(),
        };

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &sns, HYPER_EDGE),
        ));

        // hold record
        let hd = hold.wrapper(&solana, &sns, HOLD_IDENTITY);
        let hdc = hold.wrapper(&solana, &contract, HOLD_CONTRACT);
        // resolve record
        let rs = resolve.wrapper(&sns, &solana, RESOLVE);
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
        edges.push(EdgeWrapperEnum::new_resolve(rs));
    }

    Ok((next_targets, edges))
}

async fn batch_fetch_by_sns_handle(
    target: &Target,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let name = target.identity()?;
    let domain = trim_domain(name.clone());
    let owner = fetch_resolve_address(&rpc_client, &domain).await?;

    let mut next_targets = TargetProcessedList::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    match owner {
        Some(owner) => {
            let solana = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Solana,
                identity: owner.to_string(),
                uid: None,
                created_at: None,
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/profile?pubkey={}&subView=Show+All",
                    owner.to_string()
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let sns = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::SNS,
                identity: name.clone(),
                uid: None,
                created_at: None,
                display_name: Some(name.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!("https://www.sns.id/domain?domain={}", name)),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                transaction: Some("".to_string()),
                id: name.clone(),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: None,
            };

            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                system: DomainNameSystem::SNS,
                name: name.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };

            let contract = Contract {
                uuid: Uuid::new_v4(),
                category: ContractCategory::SNS,
                address: ContractCategory::SNS.default_contract_address().unwrap(),
                chain: Chain::Solana,
                symbol: Some("SNS".to_string()),
                updated_at: naive_now(),
            };

            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &solana, HYPER_EDGE),
            ));
            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &sns, HYPER_EDGE),
            ));

            // hold record
            let hd = hold.wrapper(&solana, &sns, HOLD_IDENTITY);
            let hdc = hold.wrapper(&solana, &contract, HOLD_CONTRACT);
            // resolve record
            let rs = resolve.wrapper(&sns, &solana, RESOLVE);
            edges.push(EdgeWrapperEnum::new_hold_identity(hd));
            edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
            edges.push(EdgeWrapperEnum::new_resolve(rs));

            next_targets.push(Target::Identity(Platform::Solana, owner.to_string()));
        }
        None => trace!(?target, "Owner not found"),
    }

    Ok((next_targets, edges))
}

async fn batch_fetch_by_twitter_handle(
    target: &Target,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let twitter_handle = target.identity()?;

    let mut next_targets = TargetProcessedList::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    let solana_wallet = get_twitter_registry(&rpc_client, &twitter_handle).await?;
    match solana_wallet {
        Some(solana_wallet) => {
            trace!(
                ?target,
                "Solana Wallet Founded by Twitter({}): {}",
                twitter_handle,
                solana_wallet.to_string()
            );

            let solana = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Solana,
                identity: solana_wallet.to_string(),
                uid: None,
                created_at: None,
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/profile?pubkey={}&subView=Show+All",
                    solana_wallet.to_string()
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let format_twitter = twitter_handle.to_lowercase();
            let twitter = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Twitter,
                identity: format_twitter.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_twitter.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let proof_forward: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            let proof_backward: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &solana, HYPER_EDGE),
            ));

            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &twitter, HYPER_EDGE),
            ));
            let pf = proof_forward.wrapper(&solana, &twitter, PROOF_EDGE);
            let pb = proof_backward.wrapper(&twitter, &solana, PROOF_REVERSE_EDGE);

            edges.push(EdgeWrapperEnum::new_proof_forward(pf));
            edges.push(EdgeWrapperEnum::new_proof_backward(pb));

            next_targets.push(Target::Identity(
                Platform::Solana,
                solana_wallet.to_string(),
            ));
        }
        None => trace!(?target, "Solana Wallet Not Found"),
    }

    Ok((next_targets, edges))
}

async fn fetch_by_wallet(target: &Target) -> Result<TargetProcessedList, Error> {
    let mut next_targets: TargetProcessedList = Vec::new();
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let client = make_http_client();

    let owner: String = target.identity()?;
    let verified_owner = Pubkey::from_str(&owner)?;
    let resolve_domains = fetch_resolve_domains(&rpc_client, &owner).await?;

    let mut solana = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Solana,
        identity: verified_owner.to_string(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: Some(format!(
            "https://www.sns.id/profile?pubkey={}&subView=Show+All",
            owner
        )),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    if resolve_domains.is_empty() {
        create_isolated_vertex(&client, &solana).await?;
        return Ok(vec![]);
    }

    let favourite_domain = fetch_register_favourite(&rpc_client, &owner).await?;
    match favourite_domain {
        Some(favourite_domain) => {
            let format_sol = format_domain(&favourite_domain);
            trace!(?target, "Favourite Domain Founded({})", format_sol);
            solana.reverse = Some(true); // set reverse
            solana.display_name = Some(format_sol.clone());
            let farvourite_sns = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::SNS,
                identity: format_sol.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_sol.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/domain?domain={}",
                    favourite_domain
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(true),
            };

            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                system: DomainNameSystem::SNS,
                name: format_sol.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };

            create_identity_domain_reverse_resolve_record(
                &client,
                &solana,
                &farvourite_sns,
                &resolve,
            )
            .await?;
        }
        None => trace!(?target, "Favourite Domain Not Set"),
    };

    let twitter_handle = get_handle_and_registry_key(&rpc_client, &owner).await?;
    match twitter_handle {
        Some(twitter_handle) => {
            let format_twitter = twitter_handle.to_lowercase();
            let twitter = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Twitter,
                identity: format_twitter.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_twitter.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let pf: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            let pb: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };
            create_identity_to_identity_proof_two_way_binding(&client, &solana, &twitter, &pf, &pb)
                .await?;
            next_targets.push(Target::Identity(Platform::Twitter, format_twitter.clone()))
        }
        None => trace!(?target, "Twitter Record Not Set"),
    }

    trace!(
        ?target,
        domains = resolve_domains.len(),
        "Solana Resolve Domains"
    );
    for domain in resolve_domains.iter() {
        let format_sol_handle = format_domain(domain);
        let sns = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::SNS,
            identity: format_sol_handle.clone(),
            uid: None,
            created_at: None,
            display_name: Some(format_sol_handle.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: Some(format!("https://www.sns.id/domain?domain={}", domain)),
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Solana,
            transaction: Some("".to_string()),
            id: format_sol_handle.clone(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
            expired_at: None,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Solana,
            system: DomainNameSystem::SNS,
            name: format_sol_handle.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::SNS,
            address: ContractCategory::SNS.default_contract_address().unwrap(),
            chain: Chain::Solana,
            symbol: Some("SNS".to_string()),
            updated_at: naive_now(),
        };

        create_identity_domain_resolve_record(&client, &sns, &solana, &resolve).await?;
        // ownership create `Hold_Identity` connection but only Wallet connected to HyperVertex
        create_ens_identity_ownership(&client, &solana, &sns, &hold).await?;
        create_identity_to_contract_hold_record(&client, &solana, &contract, &hold).await?;
    }

    Ok(next_targets)
}

async fn fetch_by_sns_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    let mut next_targets: TargetProcessedList = Vec::new();
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let client = make_http_client();

    let name = target.identity()?;
    let domain = trim_domain(name.clone());
    let owner = fetch_resolve_address(&rpc_client, &domain).await?;
    match owner {
        Some(owner) => {
            next_targets.push(Target::Identity(Platform::Solana, owner.to_string()));
            let solana = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Solana,
                identity: owner.to_string(),
                uid: None,
                created_at: None,
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/profile?pubkey={}&subView=Show+All",
                    owner.to_string()
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let sns = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::SNS,
                identity: name.clone(),
                uid: None,
                created_at: None,
                display_name: Some(name.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!("https://www.sns.id/domain?domain={}", name)),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                transaction: Some("".to_string()),
                id: name.clone(),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: None,
            };

            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::Solana,
                system: DomainNameSystem::SNS,
                name: name.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };

            let contract = Contract {
                uuid: Uuid::new_v4(),
                category: ContractCategory::SNS,
                address: ContractCategory::SNS.default_contract_address().unwrap(),
                chain: Chain::Solana,
                symbol: Some("SNS".to_string()),
                updated_at: naive_now(),
            };

            create_identity_domain_resolve_record(&client, &sns, &solana, &resolve).await?;
            // ownership create `Hold_Identity` connection but only Wallet connected to HyperVertex
            create_ens_identity_ownership(&client, &solana, &sns, &hold).await?;
            create_identity_to_contract_hold_record(&client, &solana, &contract, &hold).await?;
        }
        None => trace!(?target, "Owner not found"),
    }

    Ok(next_targets)
}

async fn fetch_by_twitter_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    let mut next_targets: TargetProcessedList = Vec::new();
    let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
    let client = make_http_client();

    let twitter_handle = target.identity()?;
    next_targets.push(Target::Identity(Platform::Twitter, twitter_handle.clone()));

    let solana_wallet = get_twitter_registry(&rpc_client, &twitter_handle).await?;
    match solana_wallet {
        Some(solana_wallet) => {
            next_targets.push(Target::Identity(
                Platform::Solana,
                solana_wallet.to_string(),
            ));
            trace!(
                ?target,
                "Solana Wallet Founded: {}",
                solana_wallet.to_string()
            );

            let solana = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Solana,
                identity: solana_wallet.to_string(),
                uid: None,
                created_at: None,
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: Some(format!(
                    "https://www.sns.id/profile?pubkey={}&subView=Show+All",
                    solana_wallet.to_string()
                )),
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let format_twitter = twitter_handle.to_lowercase();
            let twitter = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Twitter,
                identity: format_twitter.clone(),
                uid: None,
                created_at: None,
                display_name: Some(format_twitter.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let pf: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };

            let pb: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::SNS,
                level: ProofLevel::VeryConfident,
                record_id: None,
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
            };
            create_identity_to_identity_proof_two_way_binding(&client, &solana, &twitter, &pf, &pb)
                .await?;
        }
        None => trace!(?target, "Solana Wallet Not Found"),
    }

    Ok(next_targets)
}

lazy_static! {
    pub static ref TWITTER_VERIFICATION_AUTHORITY: Pubkey =
        Pubkey::from_str("FvPH7PrVrLGKPfqaf3xJodFTjZriqrAXXLTVWEorTFBi")
            .expect("Invalid public key");
    pub static ref TWITTER_ROOT_PARENT_REGISTRY_KEY: Pubkey =
        Pubkey::from_str("4YcexoW3r78zz16J2aqmukBLRwGq6rAvWzJpkYAXqebv")
            .expect("Invalid public key");
}

fn get_rpc_client(url: String) -> RpcClient {
    RpcClient::new(url)
}

fn format_domain(domain: &str) -> String {
    if domain.ends_with(".sol") {
        return domain.to_owned();
    }
    format!("{domain}.sol")
}

fn trim_domain(domain: String) -> String {
    if domain.ends_with(".sol") {
        return domain.trim_end_matches(".sol").to_owned();
    }
    domain
}

async fn fetch_resolve_domains(rpc_client: &RpcClient, owner: &str) -> Result<Vec<String>, Error> {
    let owner_key = Pubkey::from_str(owner)?;
    let domains = get_domains_owner(rpc_client, owner_key).await?;
    let resolve_records: Vec<String> = resolve_reverse_batch(rpc_client, &domains)
        .await?
        .into_iter()
        .filter_map(|x| x)
        .map(|x| format_domain(&x).to_string())
        .collect();
    Ok(resolve_records)
}

async fn fetch_resolve_address(
    rpc_client: &RpcClient,
    domain: &str,
) -> Result<Option<Pubkey>, Error> {
    match resolve_owner(rpc_client, &domain).await? {
        Some(owner) => Ok(Some(owner)),
        None => Ok(None),
    }
}

async fn fetch_register_favourite(
    client: &RpcClient,
    owner: &str,
) -> Result<Option<String>, Error> {
    let owner_key = Pubkey::from_str(owner)?;
    match get_favourite_domain(client, &owner_key).await? {
        None => Ok(None),
        Some(name_service_account) => match resolve_reverse(client, &name_service_account).await? {
            None => Ok(None),
            Some(reverse) => Ok(Some(reverse)),
        },
    }
}

async fn get_handle_and_registry_key(
    rpc_client: &RpcClient,
    pubkey: &str,
) -> Result<Option<String>, Error> {
    let verified_pubkey = Pubkey::from_str(pubkey)?;
    let hashed_verified_pubkey = get_hashed_name(&verified_pubkey.to_string());
    let (reverse_registry_key, _) = get_seeds_and_key(
        &spl_name_service::id(),
        hashed_verified_pubkey,
        Some(&TWITTER_VERIFICATION_AUTHORITY),
        Some(&TWITTER_ROOT_PARENT_REGISTRY_KEY),
    );

    let ascii_start_index = 33; // Starting index of "dansform"
    let handle = match resolve_name_registry(rpc_client, &reverse_registry_key).await? {
        Some((_, vec_u8)) => {
            // Skip null bytes at the beginning of the ASCII part
            let ascii_part = &vec_u8[ascii_start_index..];
            let trimmed_ascii_part = ascii_part
                .iter()
                .skip_while(|&&byte| byte == 0)
                .cloned()
                .collect::<Vec<u8>>();
            Some(deserialize_record_v2_content(
                &trimmed_ascii_part,
                Record::Twitter,
            )?)
        }
        None => None,
    };
    Ok(handle)
}

async fn get_twitter_registry(
    rpc_client: &RpcClient,
    twitter_handle: &str,
) -> Result<Option<Pubkey>, Error> {
    let hashed_twitter_handle = get_hashed_name(twitter_handle);
    let (twitter_handle_registry_key, _) = get_seeds_and_key(
        &spl_name_service::id(),
        hashed_twitter_handle,
        None, // Assuming no name class
        Some(&TWITTER_ROOT_PARENT_REGISTRY_KEY),
    );
    match resolve_name_registry(rpc_client, &twitter_handle_registry_key).await? {
        Some((header, _)) => Ok(Some(header.owner)),
        None => Ok(None),
    }
}

#[async_trait]
impl DomainSearch for Solana {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        if name == "".to_string() {
            warn!("Solana domain_search(name='') is not a valid domain name");
            return Ok(vec![]);
        }
        debug!("Solana domain_search(name={})", name);

        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            id: name.to_string(),
            updated_at: naive_now(),
        };

        let rpc_client = get_rpc_client(C.upstream.solana_rpc.rpc_url.clone());
        let owner = fetch_resolve_address(&rpc_client, name).await?;

        match owner {
            Some(owner) => {
                let sol_name = format!("{}.{}", name, EXT::Sol);
                let solana = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Solana,
                    identity: owner.to_string(),
                    uid: None,
                    created_at: None,
                    display_name: None,
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: Some(format!(
                        "https://www.sns.id/profile?pubkey={}&subView=Show+All",
                        owner.to_string()
                    )),
                    updated_at: naive_now(),
                    expired_at: None,
                    reverse: Some(false),
                };

                let sns = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::SNS,
                    identity: sol_name.clone(),
                    uid: None,
                    created_at: None,
                    display_name: Some(sol_name.clone()),
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: Some(format!("https://www.sns.id/domain?domain={}", sol_name)),
                    updated_at: naive_now(),
                    expired_at: None,
                    reverse: Some(false),
                };

                let hold: Hold = Hold {
                    uuid: Uuid::new_v4(),
                    source: DataSource::Solana,
                    transaction: Some("".to_string()),
                    id: sol_name.clone(),
                    created_at: None,
                    updated_at: naive_now(),
                    fetcher: DataFetcher::RelationService,
                    expired_at: None,
                };

                let resolve: Resolve = Resolve {
                    uuid: Uuid::new_v4(),
                    source: DataSource::Solana,
                    system: DomainNameSystem::SNS,
                    name: sol_name.clone(),
                    fetcher: DataFetcher::RelationService,
                    updated_at: naive_now(),
                };

                let collection_edge = PartOfCollection {
                    platform: Platform::SNS,
                    name: sol_name.clone(),
                    tld: EXT::Sol.to_string(),
                    status: "taken".to_string(),
                };

                // hold record
                let hd = hold.wrapper(&solana, &sns, HOLD_IDENTITY);
                // resolve record
                let rs = resolve.wrapper(&sns, &solana, RESOLVE);
                // create collection edge
                let c = collection_edge.wrapper(&domain_collection, &sns, PART_OF_COLLECTION);

                edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                edges.push(EdgeWrapperEnum::new_resolve(rs));
                edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
            }
            None => trace!("Solana domain_search(name={}) Owner not found", name),
        }

        Ok(edges)
    }
}
