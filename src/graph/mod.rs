mod edge;
mod tests;
mod vertex;

use crate::error::Error;
use async_trait::async_trait;
use gremlin_client::{
    aio::{AsyncTerminator, GremlinClient},
    process::traversal::{traversal, GraphTraversalSource},
    GremlinError, GID,
};
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
pub struct PubKeyDerivation {
    pub uuid: String,
    pub method: String,
}

#[async_trait]
trait Edge: Sized {
    // Label of edge (`addE()` parameter)
    fn label(&self) -> &'static str;

    // Save self into GraphDB.
    async fn save(&self, g: &GraphTraversalSource<AsyncTerminator>) -> Result<GID, GremlinError>;

    // Query an Edge by given direction.
    async fn find(
        g: &GraphTraversalSource<AsyncTerminator>,
        // Vertex ID
        from: &GID,
        // Vertex ID
        to: &GID,
    ) -> Result<Vec<Self>, GremlinError>;
}

#[async_trait]
trait Vertex: Sized {
    // Label of Vertex (`addV()` parameter)
    fn label(&self) -> &'static str;

    // Save self into GraphDB.
    async fn save(&self, g: &GraphTraversalSource<AsyncTerminator>) -> Result<GID, GremlinError>;

    // Query a vertex from GraphDB.
    async fn find(
        g: &GraphTraversalSource<AsyncTerminator>,
        platform: &str,
        identity: &str,
    ) -> Result<Vec<Self>, GremlinError>;
}

// TODO: should take URL from config.
const GREMLIN_URL: &str = "localhost";
pub async fn create_client() -> Result<GremlinClient, Error> {
    Ok(GremlinClient::connect(GREMLIN_URL).await?)
}

pub async fn create_traversal() -> Result<GraphTraversalSource<AsyncTerminator>, Error> {
    Ok(traversal().with_remote_async(create_client().await?))
}
