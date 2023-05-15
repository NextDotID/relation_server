pub mod edge;
mod tests;
pub mod vertex;

use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{Edge, Hold, Proof, Resolve, Wrapper},
        edge::{
            HOLD_CONTRACT, HOLD_IDENTITY, PROOF_EDGE, PROOF_REVERSE_EDGE, RESOLVE,
            RESOLVE_CONTRACT, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT,
        },
        vertex::{Contract, Identity, Vertex},
    },
    util::parse_body,
};

use http::uri::InvalidUri;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client};
use serde::{Deserialize, Serialize};
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

    #[serde(rename = "AssetGraph")]
    #[strum(serialize = "AssetGraph")]
    AssetGraph,

    #[serde(rename = "SocialGraph")]
    #[strum(serialize = "SocialGraph")]
    SocialGraph,
}

impl Graph {
    pub fn token(&self) -> String {
        use Graph::*;
        match self {
            IdentityGraph => format!("Bearer {}", C.tdb.identity_graph_token),
            AssetGraph => format!("Bearer {}", C.tdb.asset_graph_token),
            SocialGraph => format!("Bearer {}", C.tdb.social_graph_token),
        }
    }
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
    let result = match parse_body::<UpsertGraphResponse>(&mut resp).await {
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
    vertices: HashMap<String, HashMap<String, HashMap<String, Attribute>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edges: Option<
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
struct BaseResponse {
    error: bool,
    code: Option<String>,
    message: Option<String>,
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
pub struct Vertices<T>(Vec<T>);

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
            let target_vertex_id: HashMap<String, HashMap<String, Attribute>> = HashMap::from([(
                edge_wrapper.target.primary_key(),
                edge_wrapper.edge.to_attributes_map(),
            )]);
            let target_vertex_type =
                HashMap::from([(edge_wrapper.target.vertex_type(), target_vertex_id)]);
            let edge_type_map = HashMap::from([(edge_wrapper.edge.e_type(), target_vertex_type)]);
            let source_vertex_type = edges_map
                .entry(edge_wrapper.source.vertex_type())
                .or_insert(HashMap::new());
            source_vertex_type.insert(edge_wrapper.source.primary_key(), edge_type_map);

            // Insert source data
            {
                let outer_map_key = edge_wrapper.source.vertex_type().clone();
                let inner_map_key = edge_wrapper.source.primary_key().clone();

                let inner_map = vertices_map.entry(outer_map_key).or_insert(HashMap::new());
                inner_map.insert(inner_map_key, edge_wrapper.source.to_attributes_map());
            }

            // Insert target data
            {
                let outer_map_key = edge_wrapper.target.vertex_type().clone();
                let inner_map_key = edge_wrapper.target.primary_key().clone();

                let inner_map = vertices_map.entry(outer_map_key).or_insert(HashMap::new());
                inner_map.insert(inner_map_key, edge_wrapper.target.to_attributes_map());
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
    upsert_graph(client, &graph, Graph::AssetGraph).await?;
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
    upsert_graph(client, &graph, Graph::AssetGraph).await?;
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
    upsert_graph(client, &graph, Graph::AssetGraph).await?;
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
    upsert_graph(client, &graph, Graph::AssetGraph).await?;
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
    upsert_graph(client, &graph, Graph::AssetGraph).await?;
    Ok(())
}
