use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

#[derive(SimpleObject)]
// #[graphql(complex)]
pub struct Identity {
    pub uuid: uuid::Uuid,
    pub platform: String,
    pub identity: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

// #[ComplexObject]
// impl Identity {
// }

#[derive(Default)]
pub struct IdentityQuery {}

#[Object]
impl IdentityQuery {
    async fn identity(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Platform")] platform: Option<String>,
        #[graphql(desc = "Identity")] identity: Option<String>,
    ) -> Result<Identity> {
        Ok(Identity {
            uuid: uuid::Uuid::new_v4(),
            platform: platform.unwrap_or("Default Platform".to_string()),
            identity: identity.unwrap_or("Default Identity".to_string()),
            display_name: "DisplayName".into(),
            created_at: Utc::now(),
        })
    }
}
