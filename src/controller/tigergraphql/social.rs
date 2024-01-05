use crate::{
    error::{Error, Result},
    tigergraph::{
        edge::{SocialFollow, SocialGraph},
        vertex::{IdentityGraph, IdentityGraphEdge, IdentityRecord},
    },
    upstream::{DataSource, Platform},
    util::make_http_client,
};
use async_graphql::{Context, Object};
use serde::{Deserialize, Serialize};

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
impl IdentityGraph {
    async fn vertices(&self) -> &Vec<IdentityRecord> {
        &self.vertices
    }

    async fn edges(&self) -> &Vec<IdentityGraphEdge> {
        &self.edges
    }
}

#[Object]
impl IdentityGraphEdge {
    async fn source(&self) -> DataSource {
        self.source
    }

    async fn from(&self) -> &IdentityRecord {
        &self.from
    }

    async fn to(&self) -> &IdentityRecord {
        &self.to
    }
}

#[Object]
impl SocialGraph {
    async fn list(&self) -> &Option<Vec<IdentityGraph>> {
        &self.list
    }

    async fn topology(&self) -> &Option<Vec<SocialFollow>> {
        &self.topology
    }
}

#[Object]
impl SocialFollow {
    async fn source(&self) -> DataSource {
        self.source.clone()
    }

    async fn from(&self, _ctx: &Context<'_>) -> &IdentityGraph {
        &self.from
    }

    async fn to(&self, _ctx: &Context<'_>) -> &IdentityGraph {
        &self.to
    }
}
