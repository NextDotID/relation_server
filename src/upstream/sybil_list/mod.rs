mod tests;

use crate::error::Error;
use serde::Deserialize;
use serde_json::{Value, Map};
use crate::util::{timestamp_to_naive, naive_now, make_client, parse_body};
use uuid::Uuid;
use async_trait::async_trait;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};

//https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json


#[derive(Deserialize, Debug)]
pub struct SybilListItem {
    pub twitter_name: String,
    pub eth_addr: String,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug)]
pub struct VerifiedItem {
    pub twitter: TwitterItem
}

#[derive(Deserialize, Debug)]
pub struct TwitterItem {
    pub timestamp: i64,
    #[serde(rename = "tweetID")]
    pub tweet_id: String,
    pub handle: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct SybilList {}

#[async_trait]
impl Fetcher for SybilList {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> {
        let client = make_client();
        let uri: http::Uri = match format!("https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json").parse() {
            Ok(n) => n,
            Err(err) => return Err(Error::ParamError(
                format!("Uri format Error: {}", err.to_string()))),
        };
          
        let mut resp = client.get(uri).await?;
    
        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("SybilList Get error: {}", body.message),
                resp.status(),
            ));
        }

        // all records in sybil list
        let body: Map<String, Value> = parse_body(&mut resp).await?;
        
        // parse 
        let parse_body: Vec<Connection> = body
        .into_iter()
        .filter_map(|(eth_wallet_address, value)| -> Option<Connection> {
            let item: VerifiedItem = serde_json::from_value(value).ok()?;
                        
            let from: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Ethereum,
                identity: eth_wallet_address.clone(),
                created_at: None,
                display_name: Some(eth_wallet_address.clone()),
            };

            let to: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Twitter,
                identity: item.twitter.handle.clone(),
                created_at: None,
                display_name: Some(item.twitter.handle.clone()),
            };

            let pf: TempProof = TempProof {
                uuid: Uuid::new_v4(),
                method: DataSource::SybilList,
                upstream: Some(" ".to_string()),
                record_id: Some(item.twitter.tweet_id.clone()),
                created_at: Some(timestamp_to_naive(item.twitter.timestamp)), 
                last_verified_at: timestamp_to_naive(item.twitter.timestamp),
            };

            let cnn: Connection = Connection {
                from: from,
                to: to,
                proof: pf,
            };
            return Some(cnn);
        }).collect();
        
        Ok(parse_body)
    }
}

