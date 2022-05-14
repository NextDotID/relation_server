use juniper::FieldResult;
use crate::error::Error;

use super::{Context, Query};

#[derive(GraphQLInputObject, Debug)]
#[graphql(description = "Query condition available for identity")]
pub struct IdentityQuery {
    pub platform: String,
    pub identity: String,
}

#[derive(Debug, GraphQLObject)]
pub struct Identity {
    pub uuid: String,
    pub platform: String,
    pub identity: String,
    pub display_name: String,
    pub created_at: chrono::DateTime<chrono::offset::Utc>
}

impl Identity {
    pub async fn identity(context: &Context, platform: Option<String>, identity: Option<String>) -> FieldResult<Identity> {
        Ok(Identity {
            uuid: "Test UUID".into(),
            platform: platform.unwrap_or("Test Platform".into()),
            identity: identity.unwrap_or("Test Identity".into()),
            display_name: context.pool.clone(),
            created_at: chrono::Utc::now()
        })
    }
}

// graphql_object!(super::Query: Context |&self| {
//     field apiVersion() -> &str {
//         super::API_VERSION
//     }

//     field identity(&executor, query: IdentityQuery) -> FieldResult<Identity> {
//         // TODO: USE THIS
//         // let _context = executor.context();
//         // let identity = context.identity(&query.platform, &query.identity);

//         Ok(Identity {
//             uuid: "Test UUID".into(),
//             platform: query.platform,
//             identity: query.identity,
//             display_name: "Test Display Name".into(),
//             created_at: chrono::Utc::now()
//         })
//     }
// });
