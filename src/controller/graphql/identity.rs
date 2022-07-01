use aragog::DatabaseConnection;
use async_graphql::{Context, Object};
use crate::error::{Error, Result};
use crate::graph::vertex::{Identity, IdentityRecord};

#[Object]
impl IdentityRecord {
    async fn uuid(&self) -> Option<String> {
        self.uuid.map(|u| u.to_string())
    }

    async fn platform(&self) -> String {
        self.platform.to_string()
    }

    async fn identity(&self) -> String {
        self.identity.clone()
    }

    async fn display_name(&self) -> String {
        self.display_name.clone()
    }

    async fn profile_url(&self) -> Option<String> {
        self.profile_url.clone()
    }

    async fn avatar_url(&self) -> Option<String> {
        self.avatar_url.clone()
    }

    async fn created_at(&self) -> Option<i64> {
        self.created_at.map(|dt| dt.timestamp())
    }

    async fn added_at(&self) -> i64 {
        self.added_at.timestamp()
    }

    async fn updated_at(&self) -> i64 {
        self.updated_at.timestamp()
    }

    /// Neighbor identity from current. Flattened.
    async fn neighbor(
        &self,
        ctx: &Context<'_>,
        // #[graphql(
        //     desc = "Upstream source of this connection. Will search all upstreams if omitted."
        // )]
        // upstream: Option<String>,
        #[graphql(
            desc = "Depth of traversal. 1 if omitted",
        )]
        depth: Option<u16>,
    ) -> Result<Vec<IdentityRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        self.neighbors(
            db,
            depth.unwrap_or(1),
            // upstream.map(|u| DataSource::from_str(&u).unwrap_or(DataSource::Unknown))
            None
        ).await
    }
}

#[derive(Default)]
pub struct IdentityQuery;

#[Object]
impl IdentityQuery {
    /// Query an `identity` by given `platform` and `identity`.
    async fn identity(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Platform to query")] platform: String,
        #[graphql(desc = "Identity on target Platform")] identity: String,
    ) -> Result<Option<IdentityRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let platform = platform.parse()?;
        let found = Identity::find_by_platform_identity(&db, &platform, &identity).await?;

        Ok(found)
    }
}
