use crate::{
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord, EdgeWrapper, FromWithParams, Wrapper},
        vertex::{IdentitiesGraph, Identity, Vertex, VertexRecord},
        Attribute, Transfer,
    },
};

use hyper::{client::HttpConnector, Client};
use serde::{Deserialize, Serialize};
use serde_json::value::{Map, Value};
use std::collections::HashMap;
use uuid::Uuid;

// always IdentitiesGraph -> Identities
pub const HYPER_EDGE: &str = "PartOfIdentitiesGraph_Reverse";
pub const HYPER_EDGE_REVERSE: &str = "PartOfIdentitiesGraph_Reverse";
pub const IS_DIRECTED: bool = true;

/// HyperEdge
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct HyperEdge {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HyperEdgeRecord(pub EdgeRecord<HyperEdge>);

impl FromWithParams<HyperEdge> for EdgeRecord<HyperEdge> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: HyperEdge,
    ) -> Self {
        EdgeRecord {
            e_type,
            directed,
            from_id,
            from_type,
            to_id,
            to_type,
            discriminator: None,
            attributes,
        }
    }
}

impl From<EdgeRecord<HyperEdge>> for HyperEdgeRecord {
    fn from(record: EdgeRecord<HyperEdge>) -> Self {
        HyperEdgeRecord(record)
    }
}

impl std::ops::Deref for HyperEdgeRecord {
    type Target = EdgeRecord<HyperEdge>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HyperEdgeRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<HyperEdge> {
    type Target = HyperEdge;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<HyperEdge> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HyperEdgeAttribute(HashMap<String, Attribute>);

// Implement the `From` trait for converting `HyperEdgeRecord` into a `HashMap<String, Attr>`.
impl Transfer for HyperEdgeRecord {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        let attributes_map = HashMap::new();
        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let map = Map::new();
        map
    }
}

#[async_trait::async_trait]
impl Edge<IdentitiesGraph, Identity, HyperEdgeRecord> for HyperEdgeRecord {
    fn e_type(&self) -> String {
        self.e_type.clone()
    }

    fn directed(&self) -> bool {
        // TODO: query from server is the best solution
        self.directed.clone()
    }

    /// Find an edge by UUID.
    async fn find_by_uuid(
        _client: &Client<HttpConnector>,
        _uuid: &Uuid,
    ) -> Result<Option<HyperEdgeRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<IdentitiesGraph>,
        _to: &VertexRecord<Identity>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<HyperEdgeRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        _client: &Client<HttpConnector>,
        _from: &IdentitiesGraph,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }

    /// notice this function is deprecated
    async fn connect_reverse(
        &self,
        _client: &Client<HttpConnector>,
        _from: &IdentitiesGraph,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }
}

impl Wrapper<HyperEdgeRecord, IdentitiesGraph, Identity> for HyperEdge {
    fn wrapper(
        &self,
        from: &IdentitiesGraph,
        to: &Identity,
        name: &str,
    ) -> EdgeWrapper<HyperEdgeRecord, IdentitiesGraph, Identity> {
        let part_of = EdgeRecord::from_with_params(
            name.to_string(),
            IS_DIRECTED,
            from.primary_key(),
            from.vertex_type(),
            to.primary_key(),
            to.vertex_type(),
            self.to_owned(),
        );
        EdgeWrapper {
            edge: HyperEdgeRecord(part_of),
            source: from.to_owned(),
            target: to.to_owned(),
        }
    }
}
