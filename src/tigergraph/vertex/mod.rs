pub mod contract;
pub mod identity;
use async_trait::async_trait;
pub use contract::{Contract, ContractLoadFn, ContractRecord};
pub use identity::{
    ExpireTimeLoadFn, Identity, IdentityLoadFn, IdentityRecord, IdentityWithSource,
    NeighborReverseLoadFn, NeighborsResponse, OwnerLoadFn,
};
use serde::{Deserialize, Serialize};

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
