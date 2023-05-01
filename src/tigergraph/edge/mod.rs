pub mod proof;
pub use proof::{Proof, ProofRecord};

use crate::{
    config::C,
    error::Error,
    tigergraph::{
        vertex::{Identity, IdentityRecord, Vertex},
        Attribute, EdgeWrapper,
    },
};

use async_trait::async_trait;
use http::uri::InvalidUri;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// DeserializeOwned + Serialize + Clone
/// All `Edge` records.
/// // #[typetag::serde]
#[async_trait]
pub trait Edge<Source, Target, RecordType>
where
    Self: Sized + DeserializeOwned + Serialize + Clone,
    Source: Sized + Vertex,
    Target: Sized + Vertex,
    RecordType: Sized + DeserializeOwned + Serialize + Clone,
{
    fn e_type(&self) -> String;
    fn directed(&self) -> bool;

    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Source,
        to: &Target,
    ) -> Result<(), Error>;
}

pub trait Wrapper<Source, Target, RecordType>
where
    Self: Sized + DeserializeOwned + Serialize + Clone,
    Source: Sized + Vertex,
    Target: Sized + Vertex,
    RecordType: Sized + DeserializeOwned + Serialize + Clone,
{
    fn wrapper(
        &self,
        from: &Source,
        to: &Target,
        name: &str,
    ) -> EdgeWrapper<RecordType, Source, Target>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeRecord<T> {
    /// The `e_type` field of `TigerGraph` is <edge_type>.
    pub e_type: String,

    /// The `directed` field of `TigerGraph` is edge is directed - true / undirected - false.
    pub directed: bool,

    /// The `from_id` field of `TigerGraph` is <source-vertex-id>
    pub from_id: String,

    /// The `from_type` field of `TigerGraph` is <source-vertex-type>
    pub from_type: String,

    /// The `to_id` field of `TigerGraph` is <target-vertex-id>
    pub to_id: String,

    /// The `to_type` field of `TigerGraph` is <target-vertex-type>
    pub to_type: String,

    /// The attributes of <edge>
    pub attributes: T,
}

// Define a custom trait with a function that takes multiple input parameters.
pub trait FromWithParams<T> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: T,
    ) -> Self;
}
