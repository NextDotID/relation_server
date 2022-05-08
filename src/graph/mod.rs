mod tests;
mod vertex;
mod edge;

use crate::error::Error;
use gremlin_client::{aio::{GremlinClient, AsyncTerminator}, process::traversal::{traversal, GraphTraversalSource}, GValue, ToGValue};
use serde::Deserialize;

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

trait Edge {
    // Label of edge (`addE()` parameter)
    fn label(&self) -> &'static str;
}

trait Vertex {
    // Label of Vertex (`addV()` parameter)
    fn label(&self) -> &'static str;
}

// TODO: should take URL from config.
const GREMLIN_URL: &str = "localhost";
pub async fn create_client() -> Result<GremlinClient, Error> {
    Ok(GremlinClient::connect(GREMLIN_URL).await?)
}

pub async fn create_traversal() -> Result<GraphTraversalSource<AsyncTerminator>, Error> {
    Ok(traversal().with_remote_async(create_client().await?))
}
