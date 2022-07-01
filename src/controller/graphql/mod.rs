mod identity;
mod nft;
mod proof;

use self::{identity::IdentityQuery, nft::NFTQuery, proof::ProofQuery};
use async_graphql::{MergedObject, Object};

const API_VERSION: &str = "0.1";

/// Base struct of GraphQL query request.
#[derive(MergedObject, Default)]
pub struct Query(GeneralQuery, IdentityQuery, ProofQuery, NFTQuery);

#[derive(Default)]
pub struct GeneralQuery;

#[Object]
impl GeneralQuery {
    async fn ping(&self) -> &'static str {
        "Pong!"
    }

    async fn api_version(&self) -> &'static str {
        API_VERSION
    }
}
