pub mod contract;
pub mod identity;
pub mod identity_graph;
use async_trait::async_trait;
pub use contract::{Contract, ContractLoadFn, ContractRecord};
pub use identity::{
    ExpireTimeLoadFn, Identity, IdentityLoadFn, IdentityRecord, IdentityWithSource,
    NeighborReverseLoadFn, NeighborsResponse, OwnerLoadFn,
};
pub use identity_graph::{
    query_identity_graph_by_ids, Address, ExpandIdentityRecord, IdentityConnection, IdentityGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

/// All `Vertex` records.
#[async_trait]
pub trait Vertex {
    fn primary_key(&self) -> String;

    fn vertex_type(&self) -> String;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VertexRecord<T> {
    /// The `v_type` field of `TigerGraph` is <vertex-type>
    pub v_type: String,

    /// The `v_id` field of `TigerGraph` is <vertex-id>
    pub v_id: String,

    /// The attributes of <edge>
    pub attributes: T,
}

// Define a custom trait with a function that takes multiple input parameters.
pub trait FromWithParams<T> {
    fn from_with_params(v_type: String, v_id: String, attributes: T) -> Self;
}

// Define a custom trait with a function that takes multiple input parameters.
pub trait FromWithAttributes<T> {
    fn from_with_attributes(v_type: String, v_id: String, attributes: T) -> Self;
}

pub trait FromWithJsonValue<T> {
    fn from_with_json_value(v_type: String, v_id: String, attributes: T) -> Self;
}

impl FromWithJsonValue<Value> for VertexRecord<Value> {
    fn from_with_json_value(v_type: String, v_id: String, attributes: Value) -> Self {
        VertexRecord {
            v_type,
            v_id,
            attributes,
        }
    }
}
