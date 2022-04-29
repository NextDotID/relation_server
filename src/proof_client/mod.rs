mod tests;

use crate::error::Error;
use http::Response;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use crate::util::{make_client, parse_body};

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

/// Persona should be 33-bytes hexstring (`0x[0-9a-f]{66}`)
pub async fn query(base: &str, persona: &str) -> Result<ProofQueryResponse, Error> {
    let client = make_client();
    let uri = format!("{}/v1/proof?platform=nextid&identity={}", base, persona)
        .parse()
        .unwrap();
    let mut resp = client.get(uri).await?;
    if !resp.status().is_success() {
        let body: ErrorResponse = parse_body(&mut resp).await?;
        return Err(Error::General(
            format!("ProofService error: {}", body.message),
            resp.status(),
        ));
    }
    let body: ProofQueryResponse = parse_body(&mut resp).await?;
    Ok(body)
}
