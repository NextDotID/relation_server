mod tests;

use crate::error::Error;
use hyper::body;
use serde::Deserialize;
use tokio::sync::futures::Notified;
use crate::util::{naive_now, timestamp_to_naive, make_client, parse_body};
use async_trait::async_trait;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};
use uuid::Uuid;
use std::str::FromStr;
use chrono::{DateTime, NaiveDateTime};


#[derive(Deserialize, Debug)]
pub struct Rss3Response {
    pub version: String,
    pub date_updated: String,
    pub identifier: String,
    pub total: i64,
    pub list: Vec<Item>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub date_created: String,
    pub date_updated: String,
    pub related_urls: Vec<String>,
    #[serde(default)] 
    pub title: String,
    pub source: String,
    pub metadata: MetaData,
}

#[derive(Deserialize, Debug)]
pub struct MetaData {
    pub collection_address: String,
    pub collection_name: String,
    pub contract_type: String,
    pub from: String,
    pub log_index: String,
    pub network: String,
    pub proof: String,
    pub to: String,
    pub token_id: String,
    pub token_standard: String,
    pub token_symbol: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Rss3 {
    pub network: String,
    pub account: String,
    pub tags: String,
}

#[async_trait]
impl Fetcher for Rss3 {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let client = make_client();
        let uri: http::Uri = match format!("https://pregod.rss3.dev/v0.4.0/account:{}@{}/notes?tags={}", self.account, self.network, self.tags).parse() {
            Ok(n) => n,
            Err(err) => return Err(Error::ParamError(
                format!("Uri format Error: {}", err.to_string()))),
        };
  
        let mut resp = client.get(uri).await?;

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

        let parse_body: Vec<Connection> = body.list
        .into_iter()
        .filter(|p| p.metadata.to == self.account.to_lowercase())
        .filter_map(|p| -> Option<Connection> {
            let create_date_time = DateTime::parse_from_rfc3339(&p.date_created).unwrap();
            let create_naive_date_time = NaiveDateTime::from_timestamp(create_date_time.timestamp(), 0);
            let update_date_time = DateTime::parse_from_rfc3339(&p.date_updated).unwrap();
            let update_naive_date_time = NaiveDateTime::from_timestamp(update_date_time.timestamp(), 0);
            
            //println!("item :   {:?}", p);
            let from: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Ethereum,
                identity: self.account.clone(),
                created_at: None,
                display_name: Some(self.account.clone()),
            };

            let to: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Ethereum,
                identity: p.metadata.collection_address.clone(),
                created_at: Some(create_naive_date_time),
                display_name: Some(p.title.clone()),
            };

            let pf: TempProof = TempProof {
                uuid: Uuid::new_v4(),
                method: DataSource::Rss3,
                upstream: Some("https://rss3.io/network/api.html".to_string()),
                record_id: Some(p.metadata.proof.clone()),
                created_at: Some(create_naive_date_time), 
                last_verified_at: update_naive_date_time,
            };

            let cnn: Connection = Connection {
                from: from,
                to: to,
                proof: pf,
            };
            return Some(cnn);
        }).collect();
        println!("res count = {}\n", parse_body.len());

        Ok(parse_body)
    }
}
