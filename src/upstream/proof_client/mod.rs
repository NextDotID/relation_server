extern crate futures;
mod tests;

use crate::error::Error;
use crate::graph::{Vertex, Edge};
use serde::Deserialize;
use serde_json::{Value, Map};
use warp::redirect::found;
use crate::util::{timestamp_to_naive, naive_now, make_client, parse_body};
use uuid::Uuid;
use async_trait::async_trait;
use crate::upstream::{Fetcher, Platform, DataSource, Connection};
use crate::graph::{vertex::Identity, edge::Proof, new_db_connection};
use std::str::FromStr;

//use tokio_stream::{self as stream, StreamExt};
use futures::stream::{self, StreamExt, TryStreamExt};
use futures::{executor::block_on, future::join_all};

/// https://github.com/nextdotid/proof-server/blob/master/docs/api.apib
#[derive(Deserialize, Debug)]
pub struct ProofQueryResponse {
    pub pagination: ProofQueryResponsePagination,
    pub ids: Vec<ProofPersona>,
}

#[derive(Deserialize, Debug)]
pub struct ProofPersona {
    pub persona: String,
    pub proofs: Vec<ProofRecord>,
}

#[derive(Deserialize, Debug)]
pub struct ProofRecord {
    pub platform: String,
    pub identity: String,
    pub created_at: String,
    pub last_checked_at: String,
    pub is_valid: bool,
    pub invalid_reason: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofQueryResponsePagination {
    pub total: u32,
    pub per: u32,
    pub current: u32,
    pub next: u32,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct ProofClient {
    pub base: String,
    pub persona: String,
}

async fn save_item (p: ProofRecord) -> Option<Connection> {
    let db = new_db_connection().await.ok()?;

    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::NextID,
        identity: p.identity.clone(),
        created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
        display_name: p.identity.clone(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
  
    let from_record = from.create_or_update(&db).await.ok()?;

    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::from_str(p.platform.as_str()).unwrap(),
        identity: p.identity.to_string(),
        created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
        display_name: p.identity.clone(),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };
    let to_record = to.create_or_update(&db).await.ok()?;

    let pf: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::NextID,
        record_id: Some(" ".to_string()),
        created_at: Some(naive_now()), 
        last_fetched_at: naive_now(),
    };
    pf.connect(&db, &from_record, &to_record).await.ok()?;

    let cnn: Connection = Connection {
        from: from,
        to: to,
        proof: pf,
    };

    return Some(cnn);
}

#[async_trait]
impl Fetcher for ProofClient {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let client = make_client();
        let uri: http::Uri = match format!("{}/v1/proof?platform=nextid&identity={}", self.base, self.persona).parse() {
            Ok(n) => n,
            Err(err) => return Err(Error::ParamError(
                format!("Uri format Error: {}", err.to_string()))),
        };
        let mut resp = client.get(uri).await?;
    
        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Proof Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let mut body: ProofQueryResponse = parse_body(&mut resp).await?;  
        if body.pagination.total == 0 {
            return Err(Error::NoResult);
        }

        let proofs = match body.ids.pop() {
            Some(i) => i,
            None => {
                return Err(Error::NoResult); 
            }
        };
        
        // let parse_body: Vec<Connection> = proofs.proofs
        // .into_iter()
        // .filter_map(|p| -> Option<Connection> {          
            
        //     let from: Identity = Identity {
        //         uuid: Some(Uuid::new_v4()),
        //         platform: Platform::NextID,
        //         identity: p.identity.clone(),
        //         created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
        //         display_name: p.identity.clone(),
        //         added_at: naive_now(),
        //         avatar_url: None,
        //         profile_url: None,
        //         updated_at: naive_now(),
        //     };

        //     let to: Identity = Identity {
        //         uuid: Some(Uuid::new_v4()),
        //         platform: Platform::from_str(p.platform.as_str()).unwrap(),
        //         identity: p.identity.to_string(),
        //         created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
        //         display_name: p.identity.clone(),
        //         added_at: naive_now(),
        //         avatar_url: None,
        //         profile_url: None,
        //         updated_at: naive_now(),
        //     };

        //     let pf: Proof = Proof {
        //         uuid: Uuid::new_v4(),
        //         source: DataSource::NextID,
        //         record_id: Some(" ".to_string()),
        //         created_at: Some(naive_now()), 
        //         last_fetched_at: naive_now(),
        //     };

        //     let cnn: Connection = Connection {
        //         from: from,
        //         to: to,
        //         proof: pf,
        //     };
        //     return Some(cnn);
        // }).collect();

        // parse 
        let futures :Vec<_> = proofs.proofs.into_iter().map(|p| save_item(p)).collect();
        let results = join_all(futures).await;
        let parse_body: Vec<Connection> = results.into_iter().filter_map(|i|i).collect();
        Ok(parse_body)
    }
}

