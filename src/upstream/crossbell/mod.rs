#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{Hold, Resolve};
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_identity_hold_record;
use crate::tigergraph::vertex::Identity;
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{make_http_client, naive_now, option_naive_datetime_from_utc_string};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use gql_client::Client as GQLClient;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use uuid::Uuid;

#[derive(Serialize)]
struct QueryVars {
    target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct QueryResponse {
    characters: Vec<Character>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Character {
    #[serde(rename = "characterId")]
    character_id: String,
    handle: String,
    owner: String,
    primary: bool,

    #[serde(rename = "transactionHash")]
    transaction_hash: Option<String>,

    #[serde(rename = "createdAt")]
    #[serde(deserialize_with = "option_naive_datetime_from_utc_string")]
    created_at: Option<NaiveDateTime>,

    #[serde(rename = "updatedAt")]
    #[serde(deserialize_with = "option_naive_datetime_from_utc_string")]
    updated_at: Option<NaiveDateTime>,

    metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Metadata {
    content: Option<Content>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Content {
    bio: Option<String>,
    name: Option<String>,
    avatars: Option<Vec<String>>,
}

#[allow(dead_code)]
const QUERY_BY_HANDLE: &str = r#"
  query QueryByHandle($target: String!) {
    characters(where: {handle: {equals: $target}}) {
      characterId
      handle
      owner
      primary
      createdAt
      updatedAt
      transactionHash
      metadata {
        content
      }
    }
  }
"#;

const QUERY_BY_WALLET: &str = r#"
  query QueryByWallet($target: String!){
    characters(where: {owner: {equals: $target}}) {
      characterId
      handle
      owner
      primary
      createdAt
      updatedAt
      transactionHash
      metadata {
        content
      }
    }
  }
"#;

pub struct Crossbell {}

#[async_trait]
impl Fetcher for Crossbell {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target.platform()? {
            Platform::Ethereum => fetch_by_wallet(target).await,
            Platform::Crossbell => fetch_by_crossbell_handle(target).await,
            _ => Ok(vec![]),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Crossbell])
    }
}

async fn fetch_by_wallet(target: &Target) -> Result<TargetProcessedList, Error> {
    let query = QUERY_BY_WALLET.to_string();
    let target_var = target.identity()?;
    let client = GQLClient::new(&C.upstream.crossbell_api.url);
    let vars = QueryVars {
        target: target_var.to_lowercase(),
    };
    let resp = client.query_with_vars::<QueryResponse, QueryVars>(&query, vars);

    let data: Option<QueryResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), resp).await {
            Ok(resp) => match resp {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(?target, ?err, "Crossbell: Failed to fetch");
                    None
                }
            },
            Err(_) => {
                warn!(?target, "Crossbell timeout: no response in 5 seconds.");
                None
            }
        };

    if data.is_none() {
        info!(?target, "Crossbell: No result");
        return Ok(vec![]);
    }
    let res = data.unwrap();
    debug!(?target, characters = res.characters.len(), "Records found.");

    for p in res.characters.iter() {
        save_character(p).await?;
    }
    Ok(vec![Target::Identity(
        Platform::Ethereum,
        target.identity()?,
    )])
}

async fn fetch_by_crossbell_handle(target: &Target) -> Result<TargetProcessedList, Error> {
    let query = QUERY_BY_HANDLE.to_string();
    let target_var = target.identity()?;
    let handle = target_var.trim_end_matches(".csb");
    let client = GQLClient::new(&C.upstream.crossbell_api.url);
    let vars = QueryVars {
        target: handle.to_string(),
    };
    let resp = client.query_with_vars::<QueryResponse, QueryVars>(&query, vars);

    let data: Option<QueryResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), resp).await {
            Ok(resp) => match resp {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(?target, ?err, "Crossbell: Failed to fetch");
                    None
                }
            },
            Err(_) => {
                warn!(?target, "Crossbell timeout: no response in 5 seconds.");
                None
            }
        };

    if data.is_none() {
        info!(?target, "Crossbell: No result");
        return Ok(vec![]);
    }
    let res = data.unwrap();
    debug!(?target, characters = res.characters.len(), "Records found.");

    let owner = res.characters.first().unwrap().owner.clone().to_lowercase();

    for p in res.characters.iter() {
        save_character(p).await?;
    }
    Ok(vec![Target::Identity(Platform::Ethereum, owner)])
}

async fn save_character(profile: &Character) -> Result<(), Error> {
    let client = make_http_client();
    let handle = profile.handle.clone();
    let csb = format!("{}.csb", handle);
    tracing::info!("csb: {:?}", csb);
    let display_name = profile.metadata.clone().map_or(handle.clone(), |res| {
        res.content.map_or(handle.clone(), |content| {
            content.name.map_or(handle.clone(), |name| name)
        })
    });
    let avatar = profile.metadata.clone().map_or(None, |res| {
        res.content.map_or(None, |content| {
            content
                .avatars
                .map_or(None, |avatars| avatars.first().cloned())
        })
    });
    let mut crossbell = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Crossbell,
        identity: csb.clone(),
        uid: Some(profile.character_id.clone()),
        created_at: profile.created_at,
        display_name: Some(display_name),
        added_at: naive_now(),
        avatar_url: avatar,
        profile_url: Some("https://xchar.app/".to_owned() + &profile.handle.clone()),
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    let owner = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: profile.owner.clone(),
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

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Crossbell,
        transaction: profile.transaction_hash.clone(),
        id: profile.character_id.clone(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };
    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::Crossbell,
        system: DomainNameSystem::Crossbell,
        name: csb.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    if profile.primary {
        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::Crossbell,
            system: DomainNameSystem::Crossbell,
            name: csb.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        crossbell.reverse = Some(true);
        create_identity_domain_reverse_resolve_record(&client, &owner, &crossbell, &resolve)
            .await?;
    }

    create_identity_to_identity_hold_record(&client, &owner, &crossbell, &hold).await?;
    create_identity_domain_resolve_record(&client, &crossbell, &owner, &resolve).await?;

    Ok(())
}
