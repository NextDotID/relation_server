mod tests;

use std::str::FromStr;

use crate::config::C;
use crate::graph::vertex::{contract::Chain, contract::ContractCategory, Contract, Identity};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, IdentityProcessList, Platform};
use crate::util::{make_client, naive_now, parse_body};
use crate::{
    error::Error,
    graph::{edge::Own, new_db_connection},
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime};
use futures::future::join_all;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Rss3Response {
    pub version: String,
    #[serde(default)]
    pub date_updated: String,
    pub identifier: String,
    pub total: i64,
    pub list: Vec<Item>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub date_created: String,
    #[serde(default)]
    pub date_updated: String,
    pub related_urls: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub title: String,
    pub source: String,
    pub metadata: MetaData,
}

#[derive(Deserialize, Debug)]
pub struct MetaData {
    #[serde(default)]
    pub collection_address: String,
    #[serde(default)]
    pub collection_name: String,
    #[serde(default)]
    pub contract_type: String,
    pub from: String,
    #[serde(default)]
    pub log_index: String,
    pub network: String,
    pub proof: String,
    pub to: String,
    #[serde(default)]
    pub token_id: String,
    #[serde(default)]
    pub token_address: String,
    pub token_standard: String,
    pub token_symbol: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Rss3 {
    pub platform: String,
    pub identity: String,
}

async fn save_item(p: Item) -> Result<(), Error> {
    let create_date_time = DateTime::parse_from_rfc3339(&p.date_created).unwrap();
    let create_naive_date_time = NaiveDateTime::from_timestamp(create_date_time.timestamp(), 0);

    let db = new_db_connection().await?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: p.metadata.to.clone().to_lowercase(),
        created_at: Some(create_naive_date_time),
        display_name: p.metadata.to.clone().to_lowercase(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let from_record = from.create_or_update(&db).await?;

    let to: Contract = Contract {
        uuid: Uuid::new_v4(),
        category: ContractCategory::from_str(p.metadata.contract_type.as_str()).unwrap(),
        contract: p.metadata.collection_address.clone().to_lowercase(),

        chain: Chain::from_str(p.metadata.network.as_str()).unwrap(),
        symbol: Some(p.metadata.token_symbol.clone()),
        updated_at: naive_now(),
    };

    let to_record = to.create_or_update(&db).await?;

    let ownership: Own = Own {
        uuid: Uuid::new_v4(),
        source: DataSource::Rss3,
        transaction: Some(p.metadata.proof.clone()),
        token_id: p.metadata.token_id.clone(),
        connected_at: naive_now(),
    };

    ownership.connect(&db, &from_record, &to_record).await?;

    Ok(())
}

#[async_trait]
impl Fetcher for Rss3 {
    async fn fetch(&self) -> Result<IdentityProcessList, Error> {
        let client = make_client();
        let uri: http::Uri = match format!(
            "{}account:{}@{}/notes?tags=NFT",
            C.upstream.rss3_service.url, self.identity, self.platform
        )
        .parse()
        {
            Ok(n) => n,
            Err(err) => {
                return Err(Error::ParamError(format!(
                    "Uri format Error: {}",
                    err.to_string()
                )))
            }
        };

        //println!("url {:?}", uri.to_string());
        let mut resp = client.get(uri).await?;
        //println!("resp {:?}", resp.status());

        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Rss3 Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let body: Rss3Response = parse_body(&mut resp).await?;

        if body.total == 0 {
            return Err(Error::General(
                format!("rss3 Result Get Error"),
                resp.status(),
            ));
        }

        // parse
        let futures: Vec<_> = body
            .list
            .into_iter()
            .filter(|p| p.metadata.to == self.identity.to_lowercase())
            .map(|p| save_item(p))
            .collect();
        let _results = join_all(futures).await;
        //let parse_body: Vec<Connection> = results.into_iter().filter_map(|i| i).collect();

        Ok(vec![])
    }

    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)> {
        return vec![(vec![Platform::Ethereum], vec![])];
    }
}
