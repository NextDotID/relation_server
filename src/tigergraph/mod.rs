pub mod edge;
mod tests;
pub mod upsert;
pub mod vertex;

use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{
            Edge, Hold, HoldRecord, HyperEdgeRecord, Proof, ProofRecord, Resolve, ResolveRecord,
            Wrapper,
        },
        edge::{
            HOLD_CONTRACT, HOLD_IDENTITY, HYPER_EDGE_REVERSE, PROOF_EDGE, PROOF_REVERSE_EDGE,
            RESOLVE, RESOLVE_CONTRACT, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT,
        },
        vertex::{Contract, IdentitiesGraph, Identity, Vertex},
    },
    util::parse_body,
};

use http::uri::InvalidUri;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::Value;
use std::collections::HashMap;
use strum_macros::{Display, EnumIter, EnumString};
use tracing::{error, trace};

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Display,
    EnumString,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Copy,
)]
pub enum OpCode {
    /// "ignore_if_exists" or "~"
    #[default]
    #[strum(serialize = "ignore_if_exists")]
    #[serde(rename = "ignore_if_exists")]
    IgnoreIfExists,
    /// "add" or "+"
    #[serde(rename = "add")]
    #[strum(serialize = "add")]
    Add,
    /// "and" or "&"
    #[serde(rename = "and")]
    #[strum(serialize = "and")]
    And,
    /// "or" or "|"
    #[serde(rename = "or")]
    #[strum(serialize = "or")]
    Or,
    /// "max" or ">"
    #[serde(rename = "max")]
    #[strum(serialize = "max")]
    Max,
    /// "min" or "<"
    #[serde(rename = "min")]
    #[strum(serialize = "min")]
    Min,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Attribute {
    #[serde(rename = "value")]
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<OpCode>,
}

/// List of GraphName in TigerGraph.
#[derive(
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Debug,
    Display,
    PartialEq,
    Eq,
    EnumString,
    EnumIter,
    Hash,
)]
pub enum Graph {
    #[default]
    #[serde(rename = "IdentityGraph")]
    #[strum(serialize = "IdentityGraph")]
    IdentityGraph,

    #[serde(rename = "SocialGraph")]
    #[strum(serialize = "SocialGraph")]
    SocialGraph,
}

impl Graph {
    pub fn token(&self) -> String {
        use Graph::*;
        match self {
            IdentityGraph => format!("Bearer {}", C.tdb.identity_graph_token),
            SocialGraph => format!("Bearer {}", C.tdb.social_graph_token),
        }
    }
}

pub async fn delete_vertex_and_edge(
    client: &Client<HttpConnector>,
    v_id: String,
) -> Result<(), Error> {
    if v_id == "" {
        return Err(Error::ParamError("v_id is required".to_string()));
    }
    let uri: http::Uri = format!(
        "{}/query/{}/delete_vertex_and_edge?p={}&depth={}",
        C.tdb.host,
        Graph::IdentityGraph.to_string(),
        v_id.clone(),
        10, // max depth
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header("Authorization", Graph::IdentityGraph.token())
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "delete_vertex_and_edge | Fail to request: {:?}",
            err.to_string()
        ))
    })?;

    let _result = match parse_body::<BaseResponse>(&mut resp).await {
        Ok(r) => {
            if r.error {
                let err_message = format!(
                    "delete_vertex_and_edge error | Code: {:?}, Message: {:?}",
                    r.code, r.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
        }
        Err(err) => {
            let err_message = format!("delete_vertex_and_edge parse_body error: {:?}", err);
            error!(err_message);
            return Err(err);
        }
    };
    // let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    // println!("{}", json_raw);
    trace!("TigerGraph  delete_vertex_and_edge...");

    Ok(())
}

pub async fn upsert_graph(
    client: &Client<HttpConnector>,
    payload: &UpsertGraph,
    graph_name: Graph,
) -> Result<(), Error> {
    let uri: http::Uri = format!(
        "{}/graph/{}?vertex_must_exist=true",
        C.tdb.host,
        graph_name.to_string()
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let json_params = serde_json::to_string(&payload).map_err(|err| Error::JSONParseError(err))?;
    let req = hyper::Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("Authorization", graph_name.token())
        .body(Body::from(json_params))
        .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "TigerGraph | Fail to request upsert graph: {:?}",
            err.to_string()
        ))
    })?;
    let _result = match parse_body::<UpsertGraphResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err_resp: UpsertGraphResponse = parse_body(&mut resp).await?;
            let err_message = format!(
                "TigerGraph upsert error, Code: {:?}, Message: {:?}",
                err_resp.base.code, err_resp.base.message
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    // let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    // println!("{}", json_raw);
    trace!("TigerGraph UpsertGraph ...");
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertGraph {
    pub vertices: HashMap<String, HashMap<String, HashMap<String, Attribute>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<
        HashMap<
            String,
            HashMap<
                String,
                HashMap<String, HashMap<String, HashMap<String, HashMap<String, Attribute>>>>,
            >,
        >,
    >,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaseResponse {
    pub error: bool,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertGraphResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<UpsertResult>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertResult {
    accepted_vertices: i32,
    accepted_edges: i32,
    skipped_edges: Option<i32>,
    edge_vertices_not_exist: Vec<NotExist>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NotExist {
    v_type: String,
    v_id: String,
}

// Define `Vertices` struct that wraps a `Vec<T>`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vertices<T>(pub Vec<T>);

// Implement the `From` trait for converting `Vertices<T>` into vertices map.
impl<T: Clone + Vertex> From<Vertices<T>>
    for HashMap<String, HashMap<String, HashMap<String, Attribute>>>
where
    T: Transfer + Vertex,
{
    /// Convert each element in the `Vec<T>` into a key-value pair and insert it into the map.
    fn from(vertices: Vertices<T>) -> Self {
        let mut vertices_map: HashMap<String, HashMap<String, HashMap<String, Attribute>>> =
            HashMap::new();
        for (_, value) in vertices.0.into_iter().enumerate() {
            let outer_map_key = value.vertex_type().clone();
            let inner_map_key = value.primary_key().clone();

            let inner_map = vertices_map.entry(outer_map_key).or_insert(HashMap::new());
            inner_map.insert(inner_map_key, value.to_attributes_map()); // Insert inner data
        }
        vertices_map
    }
}

pub trait Transfer {
    fn to_attributes_map(&self) -> HashMap<String, Attribute>;
    fn to_json_value(&self) -> Value;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Edges<T>(pub Vec<T>);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeWrapper<T, Source, Target> {
    pub edge: T,
    pub source: Source,
    pub target: Target,
}

impl<T, Source, Target> From<Edges<EdgeWrapper<T, Source, Target>>> for UpsertGraph
where
    T: Transfer + Edge<Source, Target, T>,
    Source: Transfer + Vertex,
    Target: Transfer + Vertex,
{
    fn from(edges: Edges<EdgeWrapper<T, Source, Target>>) -> Self {
        let mut edges_map = HashMap::new();
        let mut vertices_map: HashMap<String, HashMap<String, HashMap<String, Attribute>>> =
            HashMap::new();

        for edge_wrapper in edges.0 {
            let source_vertex_type = edge_wrapper.source.vertex_type();
            let source_vertex_id = edge_wrapper.source.primary_key();
            let edge_type = edge_wrapper.edge.e_type();
            let target_vertex_type = edge_wrapper.target.vertex_type();
            let target_vertex_id = edge_wrapper.target.primary_key();
            let edge_attributes = edge_wrapper.edge.to_attributes_map();

            edges_map
                .entry(source_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .entry(source_vertex_id.clone())
                .or_insert_with(HashMap::new)
                .entry(edge_type.clone())
                .or_insert_with(HashMap::new)
                .entry(target_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .insert(target_vertex_id.clone(), edge_attributes);

            // Insert source data
            vertices_map
                .entry(source_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .insert(
                    source_vertex_id.clone(),
                    edge_wrapper.source.to_attributes_map(),
                );

            // Insert target data
            vertices_map
                .entry(target_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .insert(
                    target_vertex_id.clone(),
                    edge_wrapper.target.to_attributes_map(),
                );
        }

        UpsertGraph {
            vertices: vertices_map,
            edges: Some(edges_map),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EdgeWrapperEnum {
    ProofForward(EdgeWrapper<ProofRecord, Identity, Identity>),
    ProofBackward(EdgeWrapper<ProofRecord, Identity, Identity>),
    HoldIdentity(EdgeWrapper<HoldRecord, Identity, Identity>),
    HoldContract(EdgeWrapper<HoldRecord, Identity, Contract>),
    Resolve(EdgeWrapper<ResolveRecord, Identity, Identity>),
    ReverseResolve(EdgeWrapper<ResolveRecord, Identity, Identity>),
    ResolveContract(EdgeWrapper<ResolveRecord, Contract, Identity>),
    ReverseResolveContract(EdgeWrapper<ResolveRecord, Identity, Contract>),
    PartOfIdentitiesGraph(EdgeWrapper<HyperEdgeRecord, IdentitiesGraph, Identity>),
}

impl Transfer for EdgeWrapperEnum {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        match self {
            EdgeWrapperEnum::ProofForward(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::ProofBackward(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::HoldIdentity(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::HoldContract(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::Resolve(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::ReverseResolve(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::ResolveContract(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::ReverseResolveContract(wrapper) => wrapper.edge.to_attributes_map(),
            EdgeWrapperEnum::PartOfIdentitiesGraph(wrapper) => wrapper.edge.to_attributes_map(),
        }
    }

    fn to_json_value(&self) -> Value {
        match self {
            EdgeWrapperEnum::ProofForward(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::ProofBackward(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::HoldIdentity(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::HoldContract(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::Resolve(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::ReverseResolve(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::ResolveContract(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::ReverseResolveContract(wrapper) => wrapper.edge.to_json_value(),
            EdgeWrapperEnum::PartOfIdentitiesGraph(wrapper) => wrapper.edge.to_json_value(),
        }
    }
}

impl EdgeWrapperEnum {
    pub fn source(&self) -> &dyn Vertex {
        match self {
            EdgeWrapperEnum::ProofForward(wrapper) => &wrapper.source,
            EdgeWrapperEnum::ProofBackward(wrapper) => &wrapper.source,
            EdgeWrapperEnum::HoldIdentity(wrapper) => &wrapper.source,
            EdgeWrapperEnum::HoldContract(wrapper) => &wrapper.source,
            EdgeWrapperEnum::Resolve(wrapper) => &wrapper.source,
            EdgeWrapperEnum::ReverseResolve(wrapper) => &wrapper.source,
            EdgeWrapperEnum::ResolveContract(wrapper) => &wrapper.source,
            EdgeWrapperEnum::ReverseResolveContract(wrapper) => &wrapper.source,
            EdgeWrapperEnum::PartOfIdentitiesGraph(wrapper) => &wrapper.source,
        }
    }

    pub fn target(&self) -> &dyn Vertex {
        match self {
            EdgeWrapperEnum::ProofForward(wrapper) => &wrapper.target,
            EdgeWrapperEnum::ProofBackward(wrapper) => &wrapper.target,
            EdgeWrapperEnum::HoldIdentity(wrapper) => &wrapper.target,
            EdgeWrapperEnum::HoldContract(wrapper) => &wrapper.target,
            EdgeWrapperEnum::Resolve(wrapper) => &wrapper.target,
            EdgeWrapperEnum::ReverseResolve(wrapper) => &wrapper.target,
            EdgeWrapperEnum::ResolveContract(wrapper) => &wrapper.target,
            EdgeWrapperEnum::ReverseResolveContract(wrapper) => &wrapper.target,
            EdgeWrapperEnum::PartOfIdentitiesGraph(wrapper) => &wrapper.target,
        }
    }

    pub fn e_type(&self) -> &str {
        match self {
            EdgeWrapperEnum::ProofForward(_) => PROOF_EDGE,
            EdgeWrapperEnum::ProofBackward(_) => PROOF_REVERSE_EDGE,
            EdgeWrapperEnum::HoldIdentity(_) => HOLD_IDENTITY,
            EdgeWrapperEnum::HoldContract(_) => HOLD_CONTRACT,
            EdgeWrapperEnum::Resolve(_) => RESOLVE,
            EdgeWrapperEnum::ReverseResolve(_) => REVERSE_RESOLVE,
            EdgeWrapperEnum::ResolveContract(_) => RESOLVE_CONTRACT,
            EdgeWrapperEnum::ReverseResolveContract(_) => REVERSE_RESOLVE_CONTRACT,
            EdgeWrapperEnum::PartOfIdentitiesGraph(_) => HYPER_EDGE_REVERSE,
        }
    }
}

impl EdgeWrapperEnum {
    pub fn new_proof_forward(wrapper: EdgeWrapper<ProofRecord, Identity, Identity>) -> Self {
        EdgeWrapperEnum::ProofForward(wrapper)
    }

    pub fn new_proof_backward(wrapper: EdgeWrapper<ProofRecord, Identity, Identity>) -> Self {
        EdgeWrapperEnum::ProofBackward(wrapper)
    }

    pub fn new_hold_identity(wrapper: EdgeWrapper<HoldRecord, Identity, Identity>) -> Self {
        EdgeWrapperEnum::HoldIdentity(wrapper)
    }

    pub fn new_hold_contract(wrapper: EdgeWrapper<HoldRecord, Identity, Contract>) -> Self {
        EdgeWrapperEnum::HoldContract(wrapper)
    }

    pub fn new_resolve(wrapper: EdgeWrapper<ResolveRecord, Identity, Identity>) -> Self {
        EdgeWrapperEnum::Resolve(wrapper)
    }

    pub fn new_reverse_resolve(wrapper: EdgeWrapper<ResolveRecord, Identity, Identity>) -> Self {
        EdgeWrapperEnum::ReverseResolve(wrapper)
    }

    pub fn new_resolve_contract(wrapper: EdgeWrapper<ResolveRecord, Contract, Identity>) -> Self {
        EdgeWrapperEnum::ResolveContract(wrapper)
    }

    pub fn new_reverse_resolve_contract(
        wrapper: EdgeWrapper<ResolveRecord, Identity, Contract>,
    ) -> Self {
        EdgeWrapperEnum::ReverseResolveContract(wrapper)
    }

    pub fn new_hyper_edge(
        wrapper: EdgeWrapper<HyperEdgeRecord, IdentitiesGraph, Identity>,
    ) -> Self {
        EdgeWrapperEnum::PartOfIdentitiesGraph(wrapper)
    }
}

/// List edges.
pub type EdgeList = Vec<EdgeWrapperEnum>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchEdges(pub EdgeList);

impl From<BatchEdges> for UpsertGraph {
    fn from(edges: BatchEdges) -> Self {
        let mut edges_map = HashMap::new();
        let mut vertices_map: HashMap<String, HashMap<String, HashMap<String, Attribute>>> =
            HashMap::new();

        for edge_wrapper_enum in edges.0 {
            let source_vertex_type = edge_wrapper_enum.source().vertex_type();
            let source_vertex_id = edge_wrapper_enum.source().primary_key();
            let edge_type = edge_wrapper_enum.e_type();
            let target_vertex_type = edge_wrapper_enum.target().vertex_type();
            let target_vertex_id = edge_wrapper_enum.target().primary_key();
            let edge_attributes = edge_wrapper_enum.to_attributes_map();

            edges_map
                .entry(source_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .entry(source_vertex_id.clone())
                .or_insert_with(HashMap::new)
                .entry(edge_type.to_string())
                .or_insert_with(HashMap::new)
                .entry(target_vertex_type.clone())
                .or_insert_with(HashMap::new)
                .insert(target_vertex_id.clone(), edge_attributes);

            // downcast_ref is a method from the Any trait in Rust,
            // which allows you to safely attempt to
            // convert a reference to a trait object (&dyn Any)
            // back into a reference to a specific concrete type (&T)
            if let Some(source) = edge_wrapper_enum
                .source()
                .as_any()
                .downcast_ref::<IdentitiesGraph>()
            {
                vertices_map
                    .entry(source_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(source_vertex_id.clone(), source.to_attributes_map());
            }

            if let Some(target) = edge_wrapper_enum
                .target()
                .as_any()
                .downcast_ref::<IdentitiesGraph>()
            {
                vertices_map
                    .entry(target_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(target_vertex_id.clone(), target.to_attributes_map());
            }

            if let Some(source) = edge_wrapper_enum
                .source()
                .as_any()
                .downcast_ref::<Identity>()
            {
                vertices_map
                    .entry(source_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(source_vertex_id.clone(), source.to_attributes_map());
            }

            if let Some(target) = edge_wrapper_enum
                .target()
                .as_any()
                .downcast_ref::<Identity>()
            {
                vertices_map
                    .entry(target_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(target_vertex_id.clone(), target.to_attributes_map());
            }

            if let Some(source) = edge_wrapper_enum
                .source()
                .as_any()
                .downcast_ref::<Contract>()
            {
                vertices_map
                    .entry(source_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(source_vertex_id.clone(), source.to_attributes_map());
            }

            if let Some(target) = edge_wrapper_enum
                .target()
                .as_any()
                .downcast_ref::<Contract>()
            {
                vertices_map
                    .entry(target_vertex_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(target_vertex_id.clone(), target.to_attributes_map());
            }
        }

        UpsertGraph {
            vertices: vertices_map,
            edges: Some(edges_map),
        }
    }
}

pub async fn create_identity_to_identity_proof_two_way_binding(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    proof_forward: &Proof,
    proof_backward: &Proof,
) -> Result<(), Error> {
    let pf = proof_forward.wrapper(from, to, PROOF_EDGE);
    let pb = proof_backward.wrapper(to, from, PROOF_REVERSE_EDGE);
    // <Proof as Edge<Identity, Identity, Proof>>::reverse_e_type(&proof_backward),
    // <Proof as Edge<Identity, Identity, Proof>>::directed(&proof_backward),
    let edges = Edges(vec![pf, pb]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;

    Ok(())
}

pub async fn create_vertices(
    client: &Client<HttpConnector>,
    vertices: Vertices<Identity>,
) -> Result<(), Error> {
    let vertices_map: HashMap<String, HashMap<String, HashMap<String, Attribute>>> =
        vertices.into();
    let upsert_vertices = UpsertGraph {
        vertices: vertices_map,
        edges: None,
    };
    let graph: UpsertGraph = upsert_vertices.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_identity_to_identity_hold_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    hold: &Hold,
) -> Result<(), Error> {
    let hold_identity = hold.wrapper(from, to, HOLD_IDENTITY);
    let edges = Edges(vec![hold_identity]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_identity_to_contract_hold_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Contract,
    hold: &Hold,
) -> Result<(), Error> {
    let hold_contract = hold.wrapper(from, to, HOLD_CONTRACT);
    let edges = Edges(vec![hold_contract]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_contract_to_identity_resolve_record(
    client: &Client<HttpConnector>,
    from: &Contract,
    to: &Identity,
    reverse: &Resolve,
) -> Result<(), Error> {
    let resolve_contract = reverse.wrapper(from, to, RESOLVE_CONTRACT);
    let edges = Edges(vec![resolve_contract]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_identity_to_contract_reverse_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Contract,
    reverse: &Resolve,
) -> Result<(), Error> {
    let reverse_resolve_contract = reverse.wrapper(from, to, REVERSE_RESOLVE_CONTRACT);
    let edges = Edges(vec![reverse_resolve_contract]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_identity_domain_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    resolve: &Resolve,
) -> Result<(), Error> {
    let resolve_record = resolve.wrapper(from, to, RESOLVE);
    let edges = Edges(vec![resolve_record]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}

pub async fn create_identity_domain_reverse_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    reverse: &Resolve,
) -> Result<(), Error> {
    let reverse_record = reverse.wrapper(from, to, REVERSE_RESOLVE);
    let edges = Edges(vec![reverse_record]);
    let graph: UpsertGraph = edges.into();
    upsert_graph(client, &graph, Graph::IdentityGraph).await?;
    Ok(())
}
