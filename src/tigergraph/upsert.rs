use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{
            Edge, EdgeRecord, FromWithParams as EdgeFromWithParams, Hold, HoldRecord, Proof,
            ProofRecord, Resolve, ResolveRecord, Wrapper,
        },
        edge::{
            HOLD_CONTRACT, HOLD_IDENTITY, PROOF_EDGE, PROOF_REVERSE_EDGE, RESOLVE,
            RESOLVE_CONTRACT, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT,
        },
        vertex::{
            Contract, ContractRecord, FromWithParams, Identity, IdentityRecord, Vertex,
            CONTRACTS_NAME, IDENTITIES_NAME,
        },
        Attribute, BaseResponse, EdgeWrapper, Edges, Graph, Transfer,
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

////////////////////////////////// Upsert Only Edge Start //////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertEdge {
    edges: HashMap<
        String,
        HashMap<
            String,
            HashMap<String, HashMap<String, HashMap<String, HashMap<String, Attribute>>>>,
        >,
    >,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertEdgeResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<UpsertEdgeResult>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertEdgeResult {
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

impl<T, Source, Target> From<Edges<EdgeWrapper<T, Source, Target>>> for UpsertEdge
where
    T: Transfer + Edge<Source, Target, T>,
    Source: Transfer + Vertex,
    Target: Transfer + Vertex,
{
    fn from(edges: Edges<EdgeWrapper<T, Source, Target>>) -> Self {
        let mut edges_map = HashMap::new();
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
        }

        UpsertEdge { edges: edges_map }
    }
}

async fn upsert_edge(
    client: &Client<HttpConnector>,
    payload: &UpsertEdge,
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
    let _result = match parse_body::<UpsertEdgeResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err_resp: UpsertEdgeResponse = parse_body(&mut resp).await?;
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
    trace!("TigerGraph UpsertEdge ...");
    Ok(())
}

////////////////////////////////// Upsert Only Edge End //////////////////////////////////

////////////////////////////////// Upsert Hyper Vertex Start //////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertHyperVertex {
    from_str: String, // STRING TO GSQL JSONObject
    to_str: String,   // STRING TO GSQL JSONObject
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertHyperVertexResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<UpsertHyperVertexResult>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertHyperVertexResult {
    created_vertices: i32,
    created_hyper_vertices: Option<i32>,
}

async fn upsert_hyper_vertex(
    client: &Client<HttpConnector>,
    payload: &UpsertHyperVertex,
    graph: Graph,
) -> Result<(), Error> {
    let json_params = serde_json::to_string(payload).map_err(|err| Error::JSONParseError(err))?;
    let uri: http::Uri = format!("{}/query/{}/upsert_graph", C.tdb.host, graph.to_string())
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("Authorization", graph.token())
        .body(Body::from(json_params))
        .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "TigerGraph | Fail to request upsert_graph: {:?}",
            err.to_string()
        ))
    })?;
    let result = match parse_body::<UpsertHyperVertexResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err_resp: UpsertHyperVertexResponse = parse_body(&mut resp).await?;
            let err_message = format!(
                "TigerGraph upsert_graph error, Code: {:?}, Message: {:?}",
                err_resp.base.code, err_resp.base.message
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    trace!("TigerGraph UpsertGraph {}", json_raw);
    Ok(())
}

////////////////////////////////// Upsert Hyper Vertex End //////////////////////////////////

pub async fn create_identity_to_identity_proof_two_way_binding(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    proof_forward: &Proof,
    proof_backward: &Proof,
) -> Result<(), Error> {
    let pf = proof_forward.wrapper(from, to, PROOF_EDGE);
    let pb = proof_backward.wrapper(to, from, PROOF_REVERSE_EDGE);
    let edges = Edges(vec![pf, pb]);

    let from_record =
        IdentityRecord::from_with_params(from.primary_key(), from.vertex_type(), from.to_owned());
    let to_record =
        IdentityRecord::from_with_params(to.primary_key(), to.primary_key(), to.to_owned());
    let from_str = serde_json::to_string(&from_record).map_err(|err| Error::JSONParseError(err))?;
    let to_str = serde_json::to_string(&to_record).map_err(|err| Error::JSONParseError(err))?;

    let upsert_hyper_vertex_req = UpsertHyperVertex { from_str, to_str };
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;

    Ok(())
}

pub async fn create_identity_to_identity_hold_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    hold: &Hold,
) -> Result<(), Error> {
    Ok(())
}

pub async fn create_identity_to_contract_hold_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Contract,
    hold: &Hold,
) -> Result<(), Error> {
    Ok(())
}

pub async fn create_contract_to_identity_resolve_record(
    client: &Client<HttpConnector>,
    from: &Contract,
    to: &Identity,
    reverse: &Resolve,
) -> Result<(), Error> {
    Ok(())
}

pub async fn create_identity_to_contract_reverse_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Contract,
    reverse: &Resolve,
) -> Result<(), Error> {
    Ok(())
}

pub async fn create_identity_domain_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    resolve: &Resolve,
) -> Result<(), Error> {
    Ok(())
}

pub async fn create_identity_domain_reverse_resolve_record(
    client: &Client<HttpConnector>,
    from: &Identity,
    to: &Identity,
    reverse: &Resolve,
) -> Result<(), Error> {
    Ok(())
}
