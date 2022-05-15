use async_graphql::*;
use crate::util::DateTimeDefault;

#[derive(Default)]
pub struct Identity {
    pub uuid: String,
    pub platform: String,
    pub identity: String,
    pub display_name: String,
    pub created_at: DateTimeDefault,
}

#[Object]
impl Identity {
    // FIXME: InfiniteLoop!
    async fn identity(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Identity")]
        identity: Option<String>,
        #[graphql(desc = "Platform")]
        platform: Option<String>,
    ) -> Identity {
        Identity{
            uuid: uuid::Uuid::new_v4().to_string(),
            platform: platform.unwrap_or("Default Platform".into()),
            identity: identity.unwrap_or("Default Identity".into()),
            display_name: "Display Name".into(),
            created_at: DateTimeDefault::default(),
        }
    }
}
