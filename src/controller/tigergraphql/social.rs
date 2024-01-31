use crate::{
    error::{Error, Result},
    tigergraph::{
        edge::{FollowEdge, SocialFollow, SocialGraph},
        vertex::{IdentityGraph, IdentityGraphLoadFn, IdentityRecord},
    },
    upstream::{DataSource, Platform},
    util::make_http_client,
};
use async_graphql::{Context, Object};
use dataloader::non_cached::Loader;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialFollowResult {
    identity_graph: IdentityGraph,
}

#[derive(Default)]
pub struct SocialQuery;

#[Object]
impl SocialQuery {
    async fn social_follows(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Platform to query")] platform: String,
        #[graphql(desc = "Identity on target Platform")] identity: String,
    ) -> Result<Option<SocialFollowResult>> {
        let client = make_http_client();
        let platform: Platform = platform.parse()?;
        match IdentityGraph::find_by_platform_identity(&client, &platform, &identity).await? {
            None => {
                // TODO: fetch_all
                Ok(None)
            }
            Some(identity_graph) => Ok(Some(SocialFollowResult { identity_graph })),
        }
    }
}

#[Object]
impl SocialFollowResult {
    async fn identity_graph(&self) -> &IdentityGraph {
        &self.identity_graph
    }

    #[graphql(complexity = 1)]
    async fn follower_topology(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "`hop` relationships in a social network refers to the degrees of separation between entities.
                1 if omitted. 1-Hop (Direct Friends), 2-Hop (Friends of Friends), 3-Hop (Friends of Friends of Friends)."
        )]
        hop: Option<u16>,
    ) -> Result<Option<Vec<FollowEdge>>> {
        let client = make_http_client();
        self.identity_graph
            .follow_topology(&client, hop.unwrap_or(1), "follower")
            .await
    }

    #[graphql(complexity = 1)]
    async fn following_topology(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "`hop` relationships in a social network refers to the degrees of separation between entities.
                1 if omitted. 1-Hop (Direct Friends), 2-Hop (Friends of Friends), 3-Hop (Friends of Friends of Friends)."
        )]
        hop: Option<u16>,
    ) -> Result<Option<Vec<FollowEdge>>> {
        let client = make_http_client();
        self.identity_graph
            .follow_topology(&client, hop.unwrap_or(1), "following")
            .await
    }

    /// Users following this IdentityGraph
    #[graphql(complexity = 1)]
    async fn follower(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "`hop` relationships in a social network refers to the degrees of separation between entities.
                1 if omitted. 1-Hop (Direct Friends), 2-Hop (Friends of Friends), 3-Hop (Friends of Friends of Friends)."
        )]
        hop: Option<u16>,
    ) -> Result<Option<SocialGraph>> {
        let client = make_http_client();
        self.identity_graph
            .follow_relation(&client, hop.unwrap_or(1), "follower")
            .await
    }

    /// Users followed by this IdentityGraph
    #[graphql(complexity = 1)]
    async fn following(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "`hop` relationships in a social network refers to the degrees of separation between entities.
            1 if omitted. 1-Hop (Direct Friends), 2-Hop (Friends of Friends), 3-Hop (Friends of Friends of Friends)."
        )]
        hop: Option<u16>,
    ) -> Result<Option<SocialGraph>> {
        let client = make_http_client();
        self.identity_graph
            .follow_relation(&client, hop.unwrap_or(1), "following")
            .await
    }
}

#[Object]
impl SocialGraph {
    /// The collection of identity graph forming a social network.
    async fn list(&self, ctx: &Context<'_>) -> Result<Option<Vec<IdentityGraph>>> {
        let loader: &Loader<String, Option<IdentityGraph>, IdentityGraphLoadFn> =
            ctx.data().map_err(|err| Error::GraphQLError(err.message))?;

        let keys: Vec<String> = self.list.clone().map_or(vec![], |vec_uuid| {
            vec_uuid.into_iter().map(|k: Uuid| k.to_string()).collect()
        });
        let results: Vec<IdentityGraph> = loader
            .load_many(keys)
            .await
            .into_iter()
            .filter_map(|(_key, value)| value)
            .collect();

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results))
        }
    }

    ///The Collection of follow arrows forming a social network
    async fn topology(&self) -> &Option<Vec<SocialFollow>> {
        &self.topology
    }
}

#[Object]
impl FollowEdge {
    async fn data_source(&self) -> DataSource {
        self.follow_edge.source.clone()
    }

    async fn source(&self) -> String {
        self.follow_edge.from_id.clone()
    }

    async fn target(&self) -> String {
        self.follow_edge.to_id.clone()
    }

    async fn original_source(&self) -> &Option<IdentityRecord> {
        &self.original_from
    }

    async fn original_target(&self) -> &Option<IdentityRecord> {
        &self.original_to
    }
}

#[Object]
impl SocialFollow {
    async fn data_source(&self) -> DataSource {
        self.source.clone()
    }

    async fn source(&self, _ctx: &Context<'_>) -> String {
        self.from_id.clone()
    }

    async fn target(&self, _ctx: &Context<'_>) -> String {
        self.to_id.clone()
    }

    async fn original_source(&self, _ctx: &Context<'_>) -> String {
        self.original_from.clone()
    }

    async fn original_target(&self, _ctx: &Context<'_>) -> String {
        self.original_to.clone()
    }
}
