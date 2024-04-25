use crate::{
    error::Result,
    tigergraph::{
        edge::{Relation, RelationResult},
        vertex::{IdentityGraph, IdentityRecord},
    },
    upstream::{DataSource, Platform},
    util::make_http_client,
};

use async_graphql::{Context, Object};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraphResult {
    identity_graph: IdentityGraph,
}

#[Object]
impl IdentityGraphResult {
    async fn identity_graph(&self) -> &IdentityGraph {
        &self.identity_graph
    }

    #[graphql(complexity = 1)]
    async fn follow(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "`hop` relationships in a social network refers to the degrees of separation between entities.
                1 if omitted. 1-Hop (Direct Friends), 2-Hop (Friends of Friends), 3-Hop (Friends of Friends of Friends)."
        )]
        hop: Option<u16>,
        data_source: Option<Vec<DataSource>>,
        #[graphql(
            desc = "`limit` used to control the maximum number of records returned by query. It defaults to 100"
        )]
        limit: Option<u16>,
        #[graphql(
            desc = "`offset` determines the starting position from which the records are retrieved in query. It defaults to 0."
        )]
        offset: Option<u16>,
    ) -> Result<RelationResult> {
        let client = make_http_client();
        self.identity_graph
            .follow(
                &client,
                hop.unwrap_or(1),
                data_source,
                limit.unwrap_or(200),
                offset.unwrap_or(0),
            )
            .await
    }
}

#[Object]
impl Relation {
    async fn edge_type(&self) -> String {
        self.relation.edge_type.clone()
    }
    async fn tag(&self) -> Option<String> {
        self.relation.tag.clone()
    }
    async fn data_source(&self) -> DataSource {
        self.relation.data_source.clone()
    }

    async fn source(&self) -> String {
        self.relation.from_id.clone()
    }

    async fn target(&self) -> String {
        self.relation.to_id.clone()
    }

    async fn source_degree(&self) -> Option<i32> {
        self.source_degree.clone()
    }

    async fn target_degree(&self) -> Option<i32> {
        self.target_degree.clone()
    }

    async fn original_source(&self) -> &Option<IdentityRecord> {
        &self.original_from
    }

    async fn original_target(&self) -> &Option<IdentityRecord> {
        &self.original_to
    }
}

#[Object]
impl RelationResult {
    async fn count(&self) -> &i32 {
        &self.count
    }

    async fn relation(&self) -> &Vec<Relation> {
        &self.relation
    }
}

#[derive(Default)]
pub struct RelationQuery;

#[Object]
impl RelationQuery {
    async fn relation(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Platform to query")] platform: String,
        #[graphql(desc = "Identity on target Platform")] identity: String,
    ) -> Result<Option<IdentityGraphResult>> {
        let client = make_http_client();
        let platform: Platform = platform.parse()?;
        match IdentityGraph::find_graph_by_platform_identity(&client, &platform, &identity, None)
            .await?
        {
            None => {
                // TODO: fetch_all
                Ok(None)
            }
            Some(identity_graph) => Ok(Some(IdentityGraphResult { identity_graph })),
        }
    }
}
