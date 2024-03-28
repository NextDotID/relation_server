mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::Resolve;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_reverse_resolve_record;
use crate::tigergraph::upsert::create_isolated_vertex;
use crate::tigergraph::vertex::{Contract, Identity};
use crate::upstream::{Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem};
use crate::util::{make_client, make_http_client, naive_now, parse_body, request_with_timeout};
use async_trait::async_trait;
use hyper::{Body, Method};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::str::FromStr;
use tracing::info;
use uuid::Uuid;

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
    spl_name_service::state::{get_seeds_and_key, NameRecordHeader},
};

use super::{Fetcher, Platform, Target, TargetProcessedList};

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

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Solana, Platform::SNS, Platform::Twitter])
    }
}

async fn fetch_by_wallet(target: &Target) -> Result<TargetProcessedList, Error> {
    todo!()
}

async fn fetch_by_sns_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    todo!()
}

async fn fetch_by_twitter_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    todo!()
}

const RPC_URL: &str = "https://api.mainnet-beta.solana.com";

lazy_static! {
    pub static ref TWITTER_VERIFICATION_AUTHORITY: Pubkey =
        Pubkey::from_str("FvPH7PrVrLGKPfqaf3xJodFTjZriqrAXXLTVWEorTFBi")
            .expect("Invalid public key");
    pub static ref TWITTER_ROOT_PARENT_REGISTRY_KEY: Pubkey =
        Pubkey::from_str("4YcexoW3r78zz16J2aqmukBLRwGq6rAvWzJpkYAXqebv")
            .expect("Invalid public key");
}

fn get_rpc_client(url: Option<String>) -> RpcClient {
    match url {
        Some(url) => RpcClient::new(url),
        _ => RpcClient::new(RPC_URL.to_string()),
    }
}

fn format_domain(domain: &str) -> String {
    if domain.ends_with(".sol") {
        return domain.to_owned();
    }
    format!("{domain}.sol")
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
    // let name_service_account = ;
    match get_favourite_domain(client, &owner_key).await? {
        None => Ok(None),
        Some(name_service_account) => match resolve_reverse(client, &name_service_account).await? {
            None => Ok(None),
            Some(reverse) => Ok(Some(reverse)),
        },
    }
}

async fn fetch_reverse(rpc_client: &RpcClient, owner: &str) -> Result<Option<String>, Error> {
    let owner_key = Pubkey::from_str(owner)?;
    match resolve_reverse(rpc_client, &owner_key).await? {
        None => Ok(None),
        Some(reverse) => Ok(Some(reverse)),
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
) -> Result<Option<NameRecordHeader>, Error> {
    let hashed_twitter_handle = get_hashed_name(twitter_handle);
    let (twitter_handle_registry_key, _) = get_seeds_and_key(
        &spl_name_service::id(),
        hashed_twitter_handle,
        None, // Assuming no name class
        Some(&TWITTER_ROOT_PARENT_REGISTRY_KEY),
    );
    // Some(NameRecordHeader { parent_name: 4YcexoW3r78zz16J2aqmukBLRwGq6rAvWzJpkYAXqebv, owner: CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9, class: 11111111111111111111111111111111 })
    match resolve_name_registry(rpc_client, &twitter_handle_registry_key).await? {
        Some((header, _)) => Ok(Some(header)),
        None => Ok(None),
    }
}
