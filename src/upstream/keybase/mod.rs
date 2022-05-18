mod tests;

use crate::error::Error;
use serde::Deserialize;
use crate::util::{timestamp_to_naive, naive_now, make_client, parse_body};
use async_trait::async_trait;
use crate::upstream::{Fetcher,TempIdentity, TempProof, Platform, DataSource, Connection};
use uuid::Uuid;
use std::str::FromStr;


#[derive(Deserialize, Debug)]
pub struct KeybaseResponse {
    pub status: Status,
    pub them: Vec<PersonInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PersonInfo {
    pub id: String,
    pub basics: Basics,
    pub proofs_summary: ProofsSummary,
}

#[derive(Deserialize, Debug)]
pub struct  Status {
    pub code: i32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofsSummary {
    pub all: Vec<ProofItem>,
}

#[derive(Deserialize, Debug)]
pub struct Basics {
    pub username: String,
    pub ctime: i64,
    pub mtime: i64,
    pub id_version: i32,
    pub track_version: i32,
    pub last_id_change: i64,
    pub username_cased: String,
    pub status: i32,
    pub salt: String,
    pub eldest_seqno: i32,
}

#[derive(Deserialize, Debug)]
pub struct ProofItem {
    pub proof_type: String,
    pub nametag: String,
    pub state: i32,
    pub service_url: String,
    pub proof_url: String,
    pub sig_id: String,
    pub proof_id: String,
    pub human_url: String,
    pub presentation_group: String,
    pub presentation_tag: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Keybase {
    pub platform: String,
    pub identity: String,
}

#[async_trait]
impl Fetcher for Keybase {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let client = make_client();
        let uri = format!("https://keybase.io/_/api/1.0/user/lookup.json?{}={}&fields=proofs_summary", self.platform, self.identity)
            .parse()
            .unwrap();
        let mut resp = client.get(uri).await?;

        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let mut body: KeybaseResponse = parse_body(&mut resp).await?;  
        if body.status.code != 0 {
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.status.name),
                resp.status(),
            ));   
        }

        let person_info = body.them.pop().unwrap();
        let user_id = person_info.id; 
        let user_name = person_info.basics.username;
   
        let parse_body: Vec<Connection> = person_info.proofs_summary.all
        .into_iter()
        .filter_map(|p| -> Option<Connection> {          
            let from: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::Keybase,
                identity: user_id.clone(),
                created_at: Some(timestamp_to_naive(0)),
                display_name: Some(user_name.clone()),
            };

            let to: TempIdentity = TempIdentity {
                uuid: Uuid::new_v4(),
                platform: Platform::from_str(p.proof_type.as_str()).unwrap(),
                identity: p.nametag.clone(),
                created_at: Some(timestamp_to_naive(0)),
                display_name: Some(p.nametag.clone()),
            };

            let pf: TempProof = TempProof {
                uuid: Uuid::new_v4(),
                method: DataSource::Keybase,
                upstream: Some("https://keybase.io/docs/api/1.0/call/user/lookup".to_string()),
                record_id: Some(p.proof_id.clone()),
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