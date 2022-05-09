mod tests;

use crate::error::Error;
use serde::Deserialize;
use crate::util::{timestamp_to_naive, naive_now, make_client, parse_body};
use async_trait::async_trait;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};
use uuid::Uuid;
use std::str::FromStr;

/// https://github.com/nextdotid/proof-server/blob/master/docs/api.apib
#[derive(Deserialize, Debug)]
pub struct ProofQueryResponse {
    pub pagination: ProofQueryResponsePagination,
    pub ids: Vec<ProofPersona>,
}

#[derive(Deserialize, Debug)]
pub struct ProofPersona {
    pub persona: String,
    pub proofs: Vec<Proof>,
}

#[derive(Deserialize, Debug)]
pub struct Proof {
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

#[async_trait]
impl Fetcher for ProofClient {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let client = make_client();
    
        let uri = format!("{}/v1/proof?platform=nextid&identity={}", self.base, self.persona)
            .parse()
            .unwrap();
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

        let proofs =  body.ids.pop().unwrap().proofs;
        let parse_body: Vec<Connection> = proofs
        .into_iter()
        .filter_map(|p| -> Option<Connection> {          
            let from: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::NextID,
                identity: p.identity.clone(),
                created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
                display_name: Some(p.identity.clone()),
            };

            let to: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::from_str(p.platform.as_str()).unwrap(),
                identity: p.identity.to_string(),
                created_at: Some(timestamp_to_naive(p.created_at.to_string().parse().unwrap())),
                display_name: Some(p.identity),
            };

            let pf: TempProof = TempProof {
                uuid: Uuid::new_v4(),
                method: DataSource::SybilList,
                upstream: Some("Proof Service".to_string()),
                record_id: Some(" ".to_string()),
                created_at: Some(naive_now()), 
                last_verified_at: naive_now(),
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

