mod tests;

use gremlin_client::{aio::GremlinClient, Vertex};
use tokio_stream::StreamExt;
use crate::error::Error;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Identity {
    pub uuid: String,
    pub platform: String,
    pub identity: String,
    pub display_name: String,
    pub created_at: u128,
}

#[derive(Deserialize, Debug)]
pub struct CryptoIdentity {
    pub uuid: String,
    pub public_key: String,
    pub algorithm: String,
    pub curve: String,
    pub created_at: u128,
}

#[derive(Deserialize, Debug)]
pub struct Proof {
    pub uuid: String,
    pub method: String,
    pub upstream: String,
    pub record_id: String,
    pub created_at: u128,
    pub last_verified_at: u128,
}

#[derive(Deserialize, Debug)]
pub struct PubKeyDerivation {
    pub uuid: String,
    pub method: String,
}

// TODO: should take URL from config.
const GREMLIN_URL: &str = "localhost";

pub async fn connect() -> Result<(), Error> {
    let client = GremlinClient::connect(GREMLIN_URL).await?;
    let results = client.execute("g.V(param)", &[("param", &1)]).await?
        .filter_map(Result::ok)
        .map(|f| f.take::<Vertex>())
        .collect::<Result<Vec<Vertex>, _>>().await?;
    println!("{:?}", results);
    Ok(())
}
