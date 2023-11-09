mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::create_identity_to_identity_hold_record;
use crate::tigergraph::edge::Hold;
use crate::tigergraph::vertex::Identity;
use crate::upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList};
use crate::util::{make_http_client, naive_now};
use async_trait::async_trait;
use futures::future::join_all;
use gql_client::Client as GQLClient;
use hyper::{client::HttpConnector, Client};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
#[allow(dead_code)]
struct FarcasterProfile {
    /// Farcaster name (fname) The username for your Farcaster account.
    username: String,
    /// Display name Full name—up to 64 characters.
    displayName: Option<String>,
    /// A connected Ethereum address is associated with your Farcaster account via an off-chain proof
    /// allowing you to display NFTs and on-chain events on your profile.
    signerAddress: Option<String>,
    /// Farcaster ID (fid) The ID number for your Farcaster account.
    fid: i32,
}

#[derive(Serialize)]
struct UsernameQueryVars {
    username: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct UsernameQueryResponse {
    // farcasterProfile
    #[serde(rename = "farcasterProfile")]
    data: Vec<FarcasterProfile>,
}

#[derive(Serialize)]
struct SignerAddressQueryVars {
    signer: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct SignerAddressQueryResponse {
    // farcasterSigner
    #[serde(rename = "farcasterSigner")]
    data: Vec<FarcasterProfile>,
}

pub struct Farcaster {}

#[async_trait]
impl Fetcher for Farcaster {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        match target {
            Target::Identity(platform, identity) => {
                fetch_connections_by_platform_identity(platform, identity).await
            }
            Target::NFT(_, _, _, _) => todo!(),
        }
    }
    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Farcaster, Platform::Ethereum])
    }
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Farcaster => fetch_by_username(platform, identity).await,
        Platform::Ethereum => fetch_by_signer(platform, identity).await,
        _ => Ok(vec![]),
    }
}

async fn get_farcaster_profile_by_username(username: &str) -> Result<Vec<FarcasterProfile>, Error> {
    const QUERY_BY_NAME: &str = r#"
        query FarcasterSignerByName($username: String!) {
            farcasterProfile(username: $username) {
                username
                displayName
                signerAddress
                fid
            }
        }
    "#;
    let client = GQLClient::new(&C.upstream.datamgr_api.url);
    let vars = UsernameQueryVars {
        username: username.to_string(),
    };
    let response = client.query_with_vars::<UsernameQueryResponse, _>(QUERY_BY_NAME, vars);

    let data = match tokio::time::timeout(std::time::Duration::from_secs(5), response).await {
        Ok(response) => match response {
            Ok(response) => {
                let result = response.unwrap();
                result.data
            }
            Err(err) => {
                warn!(
                    "GraphQLError: Farcaster fetch | Failed to fetch profile using `username`: {}, error: {:?}", 
                    username,
                    err);
                vec![]
            }
        },
        Err(_) => {
            warn!("Farcaster fetch | Timeout: no response in 5 seconds.");
            vec![]
        }
    };
    Ok(data)
}

async fn get_farcaster_profile_by_signer(address: &str) -> Result<Vec<FarcasterProfile>, Error> {
    const QUERY_BY_SIGNER: &str = r#"
        query FarcasterNameBySigner($signer: String!) {
            farcasterSigner(signer: $signer) {
                username
                displayName
                signerAddress
                fid
            }
        }
    "#;
    let client = GQLClient::new(&C.upstream.datamgr_api.url);
    let vars = SignerAddressQueryVars {
        signer: address.to_string(),
    };
    let response = client.query_with_vars::<SignerAddressQueryResponse, _>(QUERY_BY_SIGNER, vars);

    let data = match tokio::time::timeout(std::time::Duration::from_secs(5), response).await {
        Ok(response) => match response {
            Ok(response) => {
                let result = response.unwrap();
                result.data
            }
            Err(err) => {
                warn!(
                    "Farcaster fetch | Failed to fetch profile using `signer`: {}, error: {:?}",
                    address, err
                );
                vec![]
            }
        },
        Err(_) => {
            warn!("Farcaster fetch | Timeout: no response in 5 seconds.");
            vec![]
        }
    };
    Ok(data)
}

async fn save_profile_ethereum(
    cli: &Client<HttpConnector>,
    profile: FarcasterProfile,
) -> Result<TargetProcessedList, Error> {
    let target_list = match profile.signerAddress {
        None => vec![], // signer address is null
        Some(signer_address) => match signer_address.as_str() {
            "" => vec![], // signer address is empty string
            &_ => {
                let eth_identity: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Ethereum,
                    identity: signer_address.to_lowercase().to_string(),
                    uid: None,
                    created_at: None,
                    display_name: None,
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: None,
                    updated_at: naive_now(),
                };
                let farcaster_identity: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Farcaster,
                    identity: profile.username.clone(),
                    uid: Some(profile.fid.clone().to_string()),
                    created_at: None,
                    display_name: profile.displayName.clone(),
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: None,
                    updated_at: naive_now(),
                };
                let hold: Hold = Hold {
                    uuid: Uuid::new_v4(),
                    source: DataSource::Farcaster,
                    transaction: None,
                    id: "".to_string(),
                    created_at: None,
                    updated_at: naive_now(),
                    fetcher: DataFetcher::DataMgrService,
                };
                // hold record
                create_identity_to_identity_hold_record(
                    cli,
                    &eth_identity,
                    &farcaster_identity,
                    &hold,
                )
                .await?;
                vec![Target::Identity(
                    Platform::Ethereum,
                    signer_address.to_lowercase().to_string(),
                )]
            }
        },
    };
    Ok(target_list)
}

async fn save_profile_signer(
    cli: &Client<HttpConnector>,
    profile: FarcasterProfile,
) -> Result<TargetProcessedList, Error> {
    let eth_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: profile
            .signerAddress
            .clone()
            .unwrap()
            .to_lowercase()
            .to_string(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let farcaster_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Farcaster,
        identity: profile.username.clone(),
        uid: Some(profile.fid.clone().to_string()),
        created_at: None,
        display_name: profile.displayName.clone(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Farcaster,
        transaction: None,
        id: "".to_string(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::DataMgrService,
    };
    // hold record
    create_identity_to_identity_hold_record(cli, &eth_identity, &farcaster_identity, &hold).await?;
    Ok(vec![Target::Identity(
        Platform::Farcaster,
        profile.username.clone(),
    )])
}

async fn fetch_by_username(
    _platform: &Platform,
    username: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let profiles = get_farcaster_profile_by_username(&username).await?;
    if profiles.is_empty() {
        return Err(Error::NoResult);
    }
    let futures: Vec<_> = profiles
        .into_iter()
        .map(|profile| save_profile_ethereum(&cli, profile))
        .collect();
    let targets: TargetProcessedList = join_all(futures)
        .await
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect();
    Ok(targets)
}

async fn fetch_by_signer(
    _platform: &Platform,
    address: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let profiles = get_farcaster_profile_by_signer(&address).await?;
    if profiles.is_empty() {
        return Err(Error::NoResult);
    }
    let futures: Vec<_> = profiles
        .into_iter()
        .map(|profile| save_profile_signer(&cli, profile))
        .collect();
    let targets: TargetProcessedList = join_all(futures)
        .await
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect();
    Ok(targets)
}
