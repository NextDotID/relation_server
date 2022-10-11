mod hold;
mod identity;
mod proof;
use self::{hold::HoldQuery, identity::IdentityQuery, proof::ProofQuery};
use async_graphql::{MergedObject, Object};
use tracing::debug;

const API_VERSION: &str = "0.1";

/// Base struct of GraphQL query request.
#[derive(MergedObject, Default)]
pub struct Query(GeneralQuery, IdentityQuery, ProofQuery, HoldQuery);

#[derive(Default)]
pub struct GeneralQuery;

pub fn show_pool_status(status: deadpool::Status) {
    debug!(
        "Connection pool status: max_size={}, size={}, available={}",
        status.max_size, status.size, status.available
    );
}

#[Object]
impl GeneralQuery {
    async fn ping(&self) -> &'static str {
        "Pong!"
    }

    async fn api_version(&self) -> &'static str {
        API_VERSION
    }
}
