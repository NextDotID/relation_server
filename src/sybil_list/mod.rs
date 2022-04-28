mod tests;

use crate::error::Error;
use http::Response;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use serde_json::{Value, Map};
use crate::util::{timestamp_to_naive, naive_now};
use uuid::Uuid;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};

//https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json
//#[derive(Deserialize, Debug)]
// pub struct SybilListVerfiedResponse{
//     pub res: Map<std::string::String, VerfiedItem>
// }

#[derive(Deserialize, Debug)]
pub struct SybilList {
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

pub async fn query(eth_addr: &str, twitter_name: &str) -> Result<SybilList, Error> {
    let client = make_client();
    let uri = format!("https://raw.githubusercontent.com/Uniswap/sybil-list/master/verified.json")
        .parse()
        .unwrap();
    let mut resp = client.get(uri).await?;
    //println!("{:?}", resp);

    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        return Err(Error::General(
            format!("Sybil List error: {}", body.message),
            resp.status(),
        ));
    }
    // all records in sybil list
    let body: Map<String, Value> = parse_body(&mut resp).await?;

    for (addr, value) in body {
        let item = serde_json::from_value::<VerfiedItem>(value).unwrap();

        if addr == eth_addr || item.twitter.handle == twitter_name {  
            let res: SybilList = SybilList{
                eth_addr: addr, 
                twitter_name: item.twitter.handle, 
                timestamp: item.twitter.timestamp,
            };
            return Ok(res);
        }
    }

    return Err(Error::NotExists)
}


impl Fetcher for SybilList {
    fn fetch(&self, url: Option<String>) -> Result<Vec<Connection>, Error> {
        //let uid: Uuid = uuid::Uuid(1);
        let from: TempIdentity = TempIdentity {
            uuid: Uuid::new_v4(),
            platform: Platform::Ethereum,
            identity: self.eth_addr.clone(),
            created_at: Some(timestamp_to_naive(self.timestamp)),
            display_name: Some(self.eth_addr.clone()),
        };

        let to: TempIdentity = TempIdentity {
            uuid: Uuid::new_v4(),
            platform: Platform::Twitter,
            identity: self.twitter_name.clone(),
            created_at: Some(timestamp_to_naive(self.timestamp)),
            display_name: Some(self.twitter_name.clone()),
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

        Ok(vec![cnn])
    }
}

