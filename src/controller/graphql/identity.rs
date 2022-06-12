use aragog::DatabaseConnection;
use async_graphql::{Context, Object};

use crate::error::{Error, Result};
use crate::graph::edge::ProofRecord;
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

    /// Query an `identity` by given `platform` and `identity`.
    async fn identity_from(
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

    async fn proofs(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "Upstream source of this connection. Param missing means searching all upstreams."
        )]
        upstream: Option<String>,
    ) -> Result<Vec<ProofRecord>> {
        todo!()
    }
}
