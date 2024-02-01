use crate::{
    error::{Error, Result},
    tigergraph::vertex::{
        query_identity_graph_by_ids, IdentityConnection, IdentityGraph, IdentityRecord,
    },
    upstream::DataSource,
    util::make_http_client,
};
use async_graphql::{Context, InputObject, Object};
use serde::{Deserialize, Serialize};

#[derive(InputObject, Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFilter {
    identity: String,
    platform: String,
}

#[derive(InputObject, Clone, Debug, Serialize, Deserialize)]
pub struct IdentityGraphFilter {
    by_graph_id: Option<Vec<String>>,
    by_identity_platform: Option<Vec<IdentityFilter>>,
    by_identity_id: Option<Vec<String>>,
}

#[derive(Default)]
pub struct IdentityGraphQuery;

#[Object]
impl IdentityGraphQuery {
    async fn query_identity_graph(
        &self,
        _ctx: &Context<'_>,
        filter: IdentityGraphFilter,
    ) -> Result<Vec<IdentityGraph>> {
        if let Some(graph_ids) = filter.by_graph_id {
            let client = make_http_client();
            Ok(query_identity_graph_by_ids(&client, graph_ids).await?)
        } else if let Some(_identity_filters) = filter.by_identity_platform {
            Ok(vec![])
        } else if let Some(_identity_filters) = filter.by_identity_id {
            Ok(vec![])
        } else {
            Err(Error::ParamMissing("Must use filter to query".to_string()))
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
