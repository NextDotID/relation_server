use std::vec;

use crate::{
    error::{Error, Result},
    tigergraph::{
        delete_vertex_and_edge,
        edge::{RelationUniqueTX, RelationUniqueTXRecord},
        vertex::{Identity, IdentityRecord},
    },
    upstream::{fetch_all, Platform, Target},
    util::make_http_client,
};
use async_graphql::{Context, Object};
use tokio::time::{sleep, Duration};
use tracing::{event, Level};

#[Object]
impl RelationUniqueTXRecord {
    async fn count(&self) -> u32 {
        self.count.clone()
    }

    async fn sum(&self) -> u32 {
        self.sum.clone()
    }

    async fn max(&self) -> u32 {
        self.max.clone()
    }

    async fn min(&self) -> u32 {
        self.min.clone()
    }

    async fn from(&self, _ctx: &Context<'_>) -> Result<IdentityRecord> {
        todo!()
    }

    /// Which `IdentityRecord` does this connection ends at.
    async fn to(&self, _ctx: &Context<'_>) -> Result<IdentityRecord> {
        todo!()
    }
}

/// Query entrypoint for `RelationUniqueTXRecord`
#[derive(Default)]
pub struct RelationQuery;

#[Object]
impl RelationQuery {
    #[tracing::instrument(level = "trace", skip(self, _ctx))]
    async fn relation(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Source Platform")] source_platform: String,
        #[graphql(desc = "Source Identity")] source_identity: String,
        #[graphql(desc = "Target Platform")] target_platform: String,
        #[graphql(desc = "Target Identity")] target_identity: String,
        #[graphql(desc = "Depth of traversal. 1 if omitted")] depth: Option<u16>,
    ) -> Result<Vec<RelationUniqueTXRecord>> {
        let client = make_http_client();
        let source_platform: Platform = source_platform.parse()?;
        let target_platform: Platform = target_platform.parse()?;
        let source_fetch = Target::Identity(source_platform, source_identity.clone());
        let target_fetch = Target::Identity(target_platform, target_identity.clone());
        let source =
            match Identity::find_by_platform_identity(&client, &source_platform, &source_identity)
                .await?
            {
                None => {
                    let fetch_result = fetch_all(vec![source_fetch], Some(3)).await;
                    if fetch_result.is_err() {
                        event!(
                            Level::WARN,
                            ?source_platform,
                            source_identity,
                            err = fetch_result.unwrap_err().to_string(),
                            "Failed to fetch"
                        );
                    }
                    Identity::find_by_platform_identity(&client, &source_platform, &source_identity)
                        .await?
                }
                Some(found) => {
                    if found.is_outdated() {
                        event!(
                            Level::DEBUG,
                            ?source_platform,
                            source_identity,
                            "Outdated. Delete and Refetching."
                        );
                        let v_id = found.v_id.clone();
                        tokio::spawn(async move {
                            // Delete and Refetch in the background
                            sleep(Duration::from_secs(10)).await;
                            delete_vertex_and_edge(&client, v_id).await?;
                            fetch_all(vec![source_fetch], Some(3)).await?;
                            Ok::<_, Error>(())
                        });
                    }
                    Some(found)
                }
            };

        let client = make_http_client();
        let target =
            match Identity::find_by_platform_identity(&client, &target_platform, &target_identity)
                .await?
            {
                None => {
                    let fetch_result = fetch_all(vec![target_fetch], Some(3)).await;
                    if fetch_result.is_err() {
                        event!(
                            Level::WARN,
                            ?target_platform,
                            target_identity,
                            err = fetch_result.unwrap_err().to_string(),
                            "Failed to fetch"
                        );
                    }
                    Identity::find_by_platform_identity(&client, &target_platform, &target_identity)
                        .await?
                }
                Some(found) => {
                    if found.is_outdated() {
                        event!(
                            Level::DEBUG,
                            ?target_platform,
                            target_identity,
                            "Outdated. Delete and Refetching."
                        );
                        let v_id = found.v_id.clone();
                        tokio::spawn(async move {
                            // Delete and Refetch in the background
                            sleep(Duration::from_secs(10)).await;
                            delete_vertex_and_edge(&client, v_id).await?;
                            fetch_all(vec![target_fetch], Some(3)).await?;
                            Ok::<_, Error>(())
                        });
                    }
                    Some(found)
                }
            };
        if source.is_none() || target.is_none() {
            return Ok(vec![]);
        }
        let client = make_http_client();
        let relation = RelationUniqueTX::relation(
            &client,
            &source.unwrap(),
            &target.unwrap(),
            depth.unwrap_or(1),
        )
        .await?;
        Ok(relation)
    }

    async fn expand(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Platform to query")] platform: String,
        #[graphql(desc = "Identity on target Platform")] identity: String,
        #[graphql(desc = "Depth of traversal. 1 if omitted")] depth: Option<u16>,
    ) -> Result<Vec<RelationUniqueTXRecord>> {
        let client = make_http_client();

        let platform: Platform = platform.parse()?;
        let target = Target::Identity(platform, identity.clone());
        let source =
            match Identity::find_by_platform_identity(&client, &platform, &identity).await? {
                None => {
                    let fetch_result = fetch_all(vec![target], Some(3)).await;
                    if fetch_result.is_err() {
                        event!(
                            Level::WARN,
                            ?platform,
                            identity,
                            err = fetch_result.unwrap_err().to_string(),
                            "Failed to fetch"
                        );
                    }
                    Identity::find_by_platform_identity(&client, &platform, &identity).await?
                }
                Some(found) => {
                    if found.is_outdated() {
                        event!(
                            Level::DEBUG,
                            ?platform,
                            identity,
                            "Outdated. Delete and Refetching."
                        );
                        let v_id = found.v_id.clone();
                        tokio::spawn(async move {
                            // Delete and Refetch in the background
                            sleep(Duration::from_secs(10)).await;
                            delete_vertex_and_edge(&client, v_id).await?;
                            fetch_all(vec![target], Some(3)).await?;
                            Ok::<_, Error>(())
                        });
                    }
                    Some(found)
                }
            };
        if source.is_none() {
            return Ok(vec![]);
        }
        let client = make_http_client();
        let expand =
            RelationUniqueTX::expand(&client, &source.unwrap(), depth.unwrap_or(1)).await?;
        Ok(expand)
    }
}
