use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{Edge, Hold, Proof, Resolve, Wrapper},
        edge::{
            HOLD_CONTRACT, HOLD_IDENTITY, PROOF_EDGE, PROOF_REVERSE_EDGE, RESOLVE,
            RESOLVE_CONTRACT, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT,
        },
        vertex::{Contract, FromWithJsonValue, Identity, Vertex},
        Attribute, BaseResponse, EdgeWrapper, Edges, Graph, Transfer,
    },
    util::parse_body,
};

use http::uri::InvalidUri;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use tracing::{error, trace};

use super::vertex::VertexRecord;

pub async fn delete_graph_inner_connection(
    client: &Client<HttpConnector>,
    v_id: String,
) -> Result<(), Error> {
    if v_id == "" {
        return Err(Error::ParamError("v_id is required".to_string()));
    }
    let uri: http::Uri = format!(
        "{}/query/{}/delete_graph_inner_connection?p={}&depth={}",
        C.tdb.host,
        Graph::SocialGraph.to_string(),
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
            "delete_graph_inner_connection | Fail to request: {:?}",
            err.to_string()
        ))
    })?;

    let _result = match parse_body::<BaseResponse>(&mut resp).await {
        Ok(r) => {
            if r.error {
                let err_message = format!(
                    "delete_graph_inner_connection error | Code: {:?}, Message: {:?}",
                    r.code, r.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
        }
        Err(err) => {
            let err_message = format!("delete_graph_inner_connection parse_body error: {:?}", err);
            error!(err_message);
            return Err(err);
        }
    };
    // let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    // println!("{}", json_raw);
    trace!("TigerGraph delete_graph_inner_connection...");

    Ok(())
}

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
    // trace!("TigerGraph upsert_edge ...");
    Ok(())
}

////////////////////////////////// Upsert Only Edge End //////////////////////////////////

////////////////////////////////// Upsert Hyper Vertex Start //////////////////////////////////

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HyperVertexWrapper<Source, Target> {
    pub source: Source,
    pub target: Target,
}

impl<Source, Target> TryFrom<HyperVertexWrapper<Source, Target>> for UpsertHyperVertex
where
    Source: Transfer + Vertex,
    Target: Transfer + Vertex,
{
    type Error = Error;
    fn try_from(warpper: HyperVertexWrapper<Source, Target>) -> Result<Self, Self::Error> {
        let from_record = VertexRecord::from_with_json_value(
            warpper.source.vertex_type(),
            warpper.source.primary_key(),
            warpper.source.to_json_value(),
        );
        let to_record = VertexRecord::from_with_json_value(
            warpper.target.vertex_type(),
            warpper.target.primary_key(),
            warpper.target.to_json_value(),
        );
        let from_str =
            serde_json::to_string(&from_record).map_err(|err| Error::JSONParseError(err))?;
        let to_str = serde_json::to_string(&to_record).map_err(|err| Error::JSONParseError(err))?;
        let updated_nanosecond = chrono::Utc::now().naive_utc().and_utc().timestamp_micros();
        Ok(UpsertHyperVertex {
            from_str,
            to_str,
            updated_nanosecond,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsolatedVertexWrapper<T> {
    pub vertex: T,
}

impl<T> TryFrom<IsolatedVertexWrapper<T>> for UpsertIsolatedVertex
where
    T: Transfer + Vertex,
{
    type Error = Error;
    fn try_from(warpper: IsolatedVertexWrapper<T>) -> Result<Self, Self::Error> {
        let vertex_record = VertexRecord::from_with_json_value(
            warpper.vertex.vertex_type(),
            warpper.vertex.primary_key(),
            warpper.vertex.to_json_value(),
        );
        let vertex_str =
            serde_json::to_string(&vertex_record).map_err(|err| Error::JSONParseError(err))?;
        let updated_nanosecond = chrono::Utc::now().naive_utc().and_utc().timestamp_micros();
        Ok(UpsertIsolatedVertex {
            vertex_str,
            updated_nanosecond,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertHyperVertex {
    pub from_str: String,        // STRING TO GSQL JSONObject
    pub to_str: String,          // STRING TO GSQL JSONObject
    pub updated_nanosecond: i64, // nanosecond
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertIsolatedVertex {
    pub vertex_str: String,      // STRING TO GSQL JSONObject
    pub updated_nanosecond: i64, // nanosecond
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpsertHyperVertexResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<UpsertHyperVertexResult>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertHyperVertexResult {
    created_vertices: i32,
    created_hyper_vertices: Option<i32>,
    final_identity_graph: Option<String>,
}

async fn upsert_isolated_vertex(
    client: &Client<HttpConnector>,
    payload: &UpsertIsolatedVertex,
    graph: Graph,
) -> Result<(), Error> {
    let json_params = serde_json::to_string(payload).map_err(|err| Error::JSONParseError(err))?;
    let uri: http::Uri = format!(
        "{}/query/{}/upsert_isolated_vertex",
        C.tdb.host,
        graph.to_string()
    )
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
            "TigerGraph | Fail to request upsert_isolated_vertex: {:?}",
            err.to_string()
        ))
    })?;
    let result = match parse_body::<UpsertHyperVertexResponse>(&mut resp).await {
        Ok(result) => {
            if result.base.error {
                let err_message = format!(
                    "TigerGraph upsert_hyper_vertex error, Code: {:?}, Message: {:?}",
                    result.base.code, result.base.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
            result
        }
        Err(err) => {
            let err_message = format!("TigerGraph upsert_graph parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    trace!("TigerGraph UpsertGraph {}", json_raw);
    Ok(())
}

async fn upsert_hyper_vertex(
    client: &Client<HttpConnector>,
    payload: &UpsertHyperVertex,
    graph: Graph,
) -> Result<(), Error> {
    let json_params = serde_json::to_string(payload).map_err(|err| Error::JSONParseError(err))?;
    let uri: http::Uri = format!(
        "{}/query/{}/upsert_hyper_vertex",
        C.tdb.host,
        graph.to_string()
    )
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
            "TigerGraph | Fail to request upsert_hyper_vertex: {:?}",
            err.to_string()
        ))
    })?;
    let result = match parse_body::<UpsertHyperVertexResponse>(&mut resp).await {
        Ok(result) => {
            if result.base.error {
                let err_message = format!(
                    "TigerGraph upsert_hyper_vertex error, Code: {:?}, Message: {:?} Request: {:?}",
                    result.base.code, result.base.message, payload,
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
            result
        }
        Err(err) => {
            let err_message = format!("TigerGraph upsert_graph parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    let json_raw = serde_json::to_string(&result).map_err(|err| Error::JSONParseError(err))?;
    trace!("TigerGraph upsert_hyper_vertex {}", json_raw);
    Ok(())
}

////////////////////////////////// Upsert Hyper Vertex End //////////////////////////////////

pub async fn create_isolated_vertex(
    client: &Client<HttpConnector>,
    v: &Identity,
) -> Result<(), Error> {
    let vertex_wrapper = IsolatedVertexWrapper {
        vertex: v.to_owned(),
    };
    let upsert_isolated_vertex_req: UpsertIsolatedVertex = vertex_wrapper.try_into()?;
    upsert_isolated_vertex(&client, &upsert_isolated_vertex_req, Graph::SocialGraph).await?;
    Ok(())
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
    let edges = Edges(vec![pf, pb]);

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };
    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
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
    let hold_identity = hold.wrapper(from, to, HOLD_IDENTITY);
    let edges = Edges(vec![hold_identity]);

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
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

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
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

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
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

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
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

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
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

    let from_to_wrapper = HyperVertexWrapper {
        source: from.to_owned(),
        target: to.to_owned(),
    };

    let upsert_hyper_vertex_req: UpsertHyperVertex = from_to_wrapper.try_into()?;
    let upsert_edge_req: UpsertEdge = edges.into();

    upsert_hyper_vertex(&client, &upsert_hyper_vertex_req, Graph::SocialGraph).await?;
    upsert_edge(&client, &upsert_edge_req, Graph::SocialGraph).await?;
    Ok(())
}
