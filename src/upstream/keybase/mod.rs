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

async fn query() -> Result<(), Error> { 
    let client = make_client();
    let uri = format!("https://keybase.io/_/api/1.0/user/lookup.json?github=fengshanshan&fields=proofs_summary")
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

    let mut body: KeybaseResponse = parse_body(&mut resp).await?;  
    println!("{:?}", body);
    Ok(())
}