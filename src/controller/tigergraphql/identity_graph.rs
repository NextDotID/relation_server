use crate::{
    error::Result,
    tigergraph::vertex::{IdentityConnection, IdentityGraph, IdentityRecord},
    upstream::{DataSource, Platform},
    util::make_http_client,
};
use async_graphql::{Context, Object};

#[derive(Default)]
pub struct IdentityGraphQuery;

#[Object]
impl IdentityGraphQuery {
    /// Identity graph from current.
    async fn identity_graph(
        &self,
        _ctx: &Context<'_>,
        #[graphql(desc = "Platform to query")] platform: String,
        #[graphql(desc = "Identity on target Platform")] identity: String,
    ) -> Result<Option<IdentityGraph>> {
        let client = make_http_client();
        let platform: Platform = platform.parse()?;
        match IdentityGraph::find_by_platform_identity(&client, &platform, &identity).await? {
            None => {
                // TODO: fetch_all
                Ok(None)
            }
            Some(identity_graph) => Ok(Some(identity_graph)),
        }
    }
}

#[Object]
impl IdentityGraph {
    /// Connecting a person’s different identifiers together, form an identity graph
    async fn graph_id(&self) -> String {
        self.graph_id.clone()
    }

    /// The set of vertices forming a identity graph.
    async fn vertices(&self) -> &Vec<IdentityRecord> {
        &self.vertices
    }

    /// The set of edges forming a identity graph.
    async fn edges(&self) -> &Vec<IdentityConnection> {
        &self.edges
    }
}

#[Object]
impl IdentityConnection {
    /// Returns edge type connects start node and end node.
    async fn edge_type(&self) -> String {
        self.edge_type.clone()
    }

    /// Returns data sources from upstreams supported by RelationService.
    async fn data_source(&self) -> DataSource {
        self.data_source
    }

    /// The start node that forms the edge.
    async fn source(&self) -> String {
        self.source.clone()
    }

    /// The end node that forms the edge.
    async fn target(&self) -> String {
        self.target.clone()
    }
}
