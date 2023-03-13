mod tests;

use crate::{
    config::C,
    error::Error,
    graph::{edge::Edge, edge::Hold, new_db_connection, vertex::Identity, vertex::Vertex},
    upstream::{DataFetcher, DataSource, Fetcher, Platform, Target, TargetProcessedList},
    util::naive_now,
};
use async_trait::async_trait;
use gql_client::Client;
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
    data: Option<FarcasterProfile>,
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
    data: Option<FarcasterProfile>,
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

async fn get_farcaster_profile_by_username(
    username: &str,
) -> Result<Option<FarcasterProfile>, Error> {
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
    let client = Client::new(&C.upstream.datamgr_api.url);
    let vars = UsernameQueryVars {
        username: username.to_string(),
    };
    let response = client
        .query_with_vars::<UsernameQueryResponse, _>(QUERY_BY_NAME, vars)
        .await;

    if response.is_err() {
        warn!(
            "Farcaster fetch | Failed to fetch profile using `username`: {}, error: {:?}",
            username,
            response.err()
        );
        return Ok(None);
    }

    let result = response.unwrap().unwrap();
    println!("{:?}", result);
    Ok(result.data)
}

async fn get_farcaster_profile_by_signer(address: &str) -> Result<Option<FarcasterProfile>, Error> {
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
    let client = Client::new(&C.upstream.datamgr_api.url);
    let vars = SignerAddressQueryVars {
        signer: address.to_string(),
    };
    let response = client
        .query_with_vars::<SignerAddressQueryResponse, _>(QUERY_BY_SIGNER, vars)
        .await;
    if response.is_err() {
        warn!(
            "Farcaster fetch | Failed to fetch profile using `signer`: {}, error: {:?}",
            address,
            response.err()
        );
        return Ok(None);
    }
    let result = response.unwrap().unwrap();
    println!("{:?}", result);
    Ok(result.data)
}

async fn fetch_by_username(
    _platform: &Platform,
    username: &str,
) -> Result<TargetProcessedList, Error> {
    warn!("fetch_by_username");
    let db = new_db_connection().await?;
    let profile = get_farcaster_profile_by_username(&username).await?;

    let target_processed_list = match profile {
        None => vec![], // profile is null
        Some(profile) => match profile.signerAddress {
            None => vec![], // signer address is null
            Some(signer_address) => match signer_address.as_str() {
                "" => vec![], // signer address is empty string
                &_ => {
                    let eth_identity: Identity = Identity {
                        uuid: Some(Uuid::new_v4()),
                        platform: Platform::Ethereum,
                        identity: signer_address.to_lowercase().to_string(),
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
                    let eth_record = eth_identity.create_or_update(&db).await?;
                    let farcaster_record = farcaster_identity.create_or_update(&db).await?;
                    hold.connect(&db, &eth_record, &farcaster_record).await?;

                    vec![Target::Identity(
                        Platform::Ethereum,
                        signer_address.to_lowercase().to_string(),
                    )]
                }
            },
        },
    };
    Ok(target_processed_list)
}

async fn fetch_by_signer(
    _platform: &Platform,
    address: &str,
) -> Result<TargetProcessedList, Error> {
    warn!("fetch_by_signer");
    let db = new_db_connection().await?;
    let profile = get_farcaster_profile_by_signer(&address).await?;

    let target_processed_list = match profile {
        None => vec![], // profile is null
        Some(profile) => {
            let eth_identity: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: profile
                    .signerAddress
                    .clone()
                    .unwrap()
                    .to_lowercase()
                    .to_string(),
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
            let eth_record = eth_identity.create_or_update(&db).await?;
            let farcaster_record = farcaster_identity.create_or_update(&db).await?;
            hold.connect(&db, &eth_record, &farcaster_record).await?;

            vec![Target::Identity(Platform::Farcaster, address.to_string())]
        }
    };
    Ok(target_processed_list)
}
