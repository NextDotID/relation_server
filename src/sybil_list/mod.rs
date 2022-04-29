mod tests;

use crate::error::Error;
use http::Response;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use serde_json::{Value, Map};
use crate::util::{timestamp_to_naive, naive_now};
use uuid::Uuid;
use async_trait::async_trait;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};

//https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json
//#[derive(Deserialize, Debug)]
// type SybilListVerfiedResponse = Map<String, VerfiedItem>;

#[derive(Deserialize, Debug)]
pub struct SybilListItem {
    pub twitter_name: String,
    pub eth_addr: String,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug)]
pub struct VerfiedItem {
    pub twitter: TwitterItem
}

#[derive(Deserialize, Debug)]
pub struct TwitterItem {
    pub timestamp: i64,
    pub tweetID: String,
    pub handle: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}


pub fn make_client() -> Client<HttpsConnector<HttpConnector>> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    client
}

async fn parse_body<T>(resp: &mut Response<Body>) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut body_bytes: Vec<u8> = vec![];
    while let Some(chunk) = resp.body_mut().data().await {
        let mut chunk_bytes = chunk.unwrap().to_vec();
        body_bytes.append(&mut chunk_bytes);
    }
    let body = std::str::from_utf8(&body_bytes).unwrap();

    Ok(serde_json::from_str(&body)?)
}

pub struct SybilList {}

#[async_trait]
impl Fetcher for SybilList {
    async fn fetch(&self, url: Option<String>) -> Result<Vec<Connection>, Error> {
        let client = make_client();
        let uri = format!("https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json")
            .parse()
            .unwrap();
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

        let mut vec: Vec<Connection> = Vec::new(); 
        for (addr, value) in body {
            let item = match serde_json::from_value::<VerfiedItem>(value) {
                Ok(item) => item,
                Err(_) => continue,
            };
            
            let from: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Ethereum,
                identity: addr.clone(),
                created_at: Some(timestamp_to_naive(item.twitter.timestamp)),
                display_name: Some(addr.clone()),
            };

            let to: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Twitter,
                identity: item.twitter.handle.clone(),
                created_at: Some(timestamp_to_naive(item.twitter.timestamp)),
                display_name: Some(item.twitter.handle.clone()),
            };

            let pf: TempProof = TempProof {
                uuid: Uuid::new_v4(),
                method: DataSource::SybilList,
                upstream: Some(" ".to_string()),
                record_id: Some(" ".to_string()),
                created_at: Some(naive_now()), 
                last_verified_at: naive_now(),
            };

            let cnn: Connection = Connection {
                from: from,
                to: to,
                proof: pf,
            };
            vec.push(cnn)
        }
        println!("len {}\n", vec.len());
        Ok(vec)
    }
}

