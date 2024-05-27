#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::Hold;
use crate::tigergraph::upsert::create_identity_to_contract_hold_record;
use crate::tigergraph::vertex::{Contract, Identity};
use crate::tigergraph::EdgeList;
use crate::upstream::{
    Chain, ContractCategory, DataFetcher, DataSource, Fetcher, Platform, Target,
    TargetProcessedList,
};
use crate::util::{make_http_client, naive_now};

use async_trait::async_trait;
use gql_client::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct EthQueryResponseEns {
    ens: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct EthQueryResponse {
    addrs: Vec<EthQueryResponseEns>,
}

#[derive(Serialize)]
pub struct EthQueryVars<'a> {
    addr: &'a str,
}

#[derive(Serialize)]
pub struct ENSQueryVars {
    ens: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct EnsQueryResponse {
    addrs: Vec<EnsQueryResponseAddress>,
}

#[derive(Deserialize, Debug)]
pub struct EnsQueryResponseAddress {
    address: String,
}

pub struct Knn3 {}

#[async_trait]
impl Fetcher for Knn3 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(_, identity) => fetch_ens_by_eth_wallet(identity).await,
            Target::NFT(_, _, _, id) => fetch_eth_wallet_by_ens(id).await,
        }
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }
        Ok((vec![], vec![]))
    }

    fn can_fetch(_target: &Target) -> bool {
        // TODO: temporarily disable KNN3 fetcher
        false
        // target.in_platform_supported(vec![Platform::Ethereum])
        //     || target.in_nft_supported(vec![ContractCategory::ENS], vec![Chain::Ethereum])
    }
}

/// Use ethereum address to fetch NFTs (especially ENS).
async fn fetch_ens_by_eth_wallet(identity: &str) -> Result<TargetProcessedList, Error> {
    let query = r#"
        query EnsByAddressQuery($addr: String!){
            addrs(where: { address: $addr }) {
                ens
            }
        }
    "#;

    let client = Client::new(C.upstream.knn3_service.url.clone());
    let vars = EthQueryVars {
        addr: &identity.to_lowercase(), // Yes, KNN3 is case-sensitive.
    };

    let resp = client.query_with_vars(query, vars);
    let data: Option<EthQueryResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), resp).await {
            Ok(resp) => match resp {
                Ok(resp) => {
                    let res = resp.unwrap();
                    res
                }
                Err(err) => {
                    warn!(
                        "KNN3 fetch | Failed to fetch addrs: {}, err: {:?}",
                        identity, err
                    );
                    None
                }
            },
            Err(_) => {
                warn!("KNN3 fetch | Timeout: no response in 5 seconds.");
                None
            }
        };

    if data.is_none() {
        info!("KNN3 fetch | address: {} cannot find any result", identity);
        return Ok(vec![]);
    }
    let res = data.unwrap();
    if res.addrs.is_empty() {
        info!("KNN3 fetch | address: {} cannot find any result", identity);
        return Ok(vec![]);
    }

    let ens_vec = res.addrs.first().unwrap();
    let cli = make_http_client();

    for ens in ens_vec.ens.iter() {
        let from: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: identity.to_lowercase(),
            uid: None,
            created_at: None,
            // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
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
            transaction: None,
            id: ens.to_string(),
            source: DataSource::Knn3,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
            expired_at: None,
        };
        // hold record
        create_identity_to_contract_hold_record(&cli, &from, &to, &ownership).await?;
    }
    Ok(ens_vec
        .ens
        .iter()
        .map(|ens| {
            Target::NFT(
                Chain::Ethereum,
                ContractCategory::ENS,
                ContractCategory::ENS.default_contract_address().unwrap(),
                ens.clone(),
            )
        })
        .collect())
}

async fn fetch_eth_wallet_by_ens(id: &str) -> Result<TargetProcessedList, Error> {
    let query = r#"
        query AddressByENSQuery($ens: [String]){
            addrs(where: { ens: $ens }) {
                address
            }
        }
    "#;
    let client = Client::new(C.upstream.knn3_service.url.clone());
    let vars = ENSQueryVars {
        ens: vec![id.to_string()],
    };
    let response = client.query_with_vars::<EnsQueryResponse, _>(query, vars);

    let data: Option<EnsQueryResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), response).await {
            Ok(response) => match response {
                Ok(response) => response,
                Err(err) => {
                    warn!(
                        "KNN3 fetch | Failed to fetch addrs using ENS: {}, error: {:?}",
                        id, err
                    );
                    None
                }
            },
            Err(_) => {
                warn!("KNN3 fetch | Timeout: no response in 5 seconds.");
                None
            }
        };

    if data.is_none() {
        info!("KNN3 fetch | ENS {} has no result", id);
        return Ok(vec![]);
    }
    let result = data.unwrap();
    if result.addrs.is_empty() {
        info!("KNN3 fetch | ENS {} has no result", id);
        return Ok(vec![]);
    }

    // NOTE: not sure if this result must have one and only one.
    let address = result.addrs.first().unwrap().address.clone();
    let from = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: address.to_lowercase(),
        uid: None,
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        profile_url: None,
        avatar_url: None,
        created_at: None,
        added_at: naive_now(),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };
    let to = Contract {
        uuid: Uuid::new_v4(),
        updated_at: naive_now(),
        category: ContractCategory::ENS,
        address: ContractCategory::ENS.default_contract_address().unwrap(),
        chain: Chain::Ethereum,
        symbol: None,
    };
    let hold = Hold {
        uuid: Uuid::new_v4(),
        transaction: None,
        id: id.into(),
        source: DataSource::Knn3,
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };
    // hold record
    let cli = make_http_client();
    create_identity_to_contract_hold_record(&cli, &from, &to, &hold).await?;

    Ok(vec![Target::Identity(Platform::Ethereum, address)])
}
