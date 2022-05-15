mod identity;
mod proof;

use async_graphql::{EmptyMutation, EmptySubscription, Object, MergedObject};

const API_VERSION: &str = "1.0";

pub struct Context {
    /// Real GraphDB upstream.
    /// TODO: replace it with a real database.
    pub pool: String,
}

/// Base struct of GraphQL query request.
#[derive(MergedObject, Default)]
pub struct Query(DefaultQuery, identity::Identity);

#[derive(Default)]
pub struct DefaultQuery;

#[Object]
impl DefaultQuery {
    async fn ping(&self) -> &'static str {
        "pong"
    }

    async fn api_version(&self) -> &'static str {
        API_VERSION
    }

    // async fn identity(context: &Context, platform: Option<String>, identity: Option<String>) -> FieldResult<Identity> {
    //     Identity::identity(context, platform, identity).await
    // }
}

type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;
