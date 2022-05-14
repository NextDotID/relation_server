mod proof;
mod identity;

const API_VERSION: &str = "1.0";

pub struct Context {
    /// Real GraphDB upstream.
    /// TODO: replace it with a real database.
    pub pool: String,
}
impl juniper::Context for Context {}

/// Base struct of GraphQL query request.
pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn ping() -> &'static str {
        "pong"
    }

    fn api_version() -> &'static str {
        API_VERSION
    }

    async fn identity(context: &Context, platform: Option<String>, identity: Option<String>) -> FieldResult<Identity> {
        Identity::identity(context, platform, identity).await
    }
}

// /// Base struct of GraphQL Mutation query.
// struct Mutation;

use juniper::FieldResult;

use self::identity::Identity;

type Schema = juniper::RootNode<'static, Query, (), ()>;
