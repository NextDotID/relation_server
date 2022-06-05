mod identity;
mod proof;

use async_graphql::{EmptyMutation, EmptySubscription, MergedObject, Object};

const API_VERSION: &str = "1.0";

pub struct Context {
    /// Real GraphDB upstream.
    /// TODO: replace it with a real database.
    pub pool: String,
}

/// Base struct of GraphQL query request.
#[derive(MergedObject, Default)]
pub struct Query(GeneralQuery);

#[derive(Default)]
pub struct GeneralQuery;

#[Object]
impl GeneralQuery {
    async fn ping(&self) -> &'static str {
        "pong"
    }

    async fn api_version(&self) -> &'static str {
        API_VERSION
    }
}

type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;
