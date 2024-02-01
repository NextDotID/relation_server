use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{FollowEdge, SocialFollow, SocialGraph},
        vertex::IdentityRecord,
        BaseResponse, Graph,
    },
    upstream::{DataSource, Platform},
    util::parse_body,
};
use dataloader::BatchFn;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraph {
    pub graph_id: String,
    pub vertices: Vec<IdentityRecord>,
    pub edges: Vec<IdentityConnection>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct IdentityGraphResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<IdentityGraph>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConnection {
    pub data_source: DataSource,
    #[serde(rename = "source_v")]
    pub source: String,
    #[serde(rename = "target_v")]
    pub target: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationFollowResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<SocialGraph>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FollowTopologyResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<FollowTopology>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowTopology {
    pub follow_edges: Vec<SocialFollow>,
    pub original_vertices: Vec<IdentityRecord>,
}

impl IdentityGraph {
    pub async fn find_by_platform_identity(
        client: &Client<HttpConnector>,
        platform: &Platform,
        identity: &str,
    ) -> Result<Option<IdentityGraph>, Error> {
        let p = format!("{},{}", platform, identity);
        let encoded_p = urlencoding::encode(&p);
        let uri: http::Uri = format!(
            "{}/query/{}/find_identity_graph_by_vertex?p={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            encoded_p,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!(
                "query find_identity_graph_by_vertex?p={} Uri format Error | {}",
                p, _err
            ))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!(
                    "query find_identity_graph_by_vertex ParamError Error {}",
                    _err
                ))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query find_identity_graph_by_vertex | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<IdentityGraphResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query find_identity_graph_by_vertex error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r.results.and_then(|vec_res| vec_res.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query find_identity_graph_by_vertex parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }

    pub async fn follow_topology(
        &self,
        client: &Client<HttpConnector>,
        hop: u16,
        follow_type: &str,
    ) -> Result<Option<Vec<FollowEdge>>, Error> {
        let uri: http::Uri = format!(
            "{}/query/{}/follow_topology?g={}&follow_type={}&hop={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            self.graph_id.to_string(),
            follow_type.to_string(),
            hop,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!("query follow_topology Uri format Error | {}", _err))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!("query follow_topology ParamError Error {}", _err))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query follow_topology | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<FollowTopologyResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query follow_relation error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                if let Some(topology) = r.results.and_then(|res| res.first().cloned()) {
                    let identity_map: HashMap<String, IdentityRecord> = topology
                        .original_vertices
                        .into_iter()
                        .map(|record| (record.v_id.clone(), record))
                        .collect();

                    let follow_edges: Vec<FollowEdge> = topology
                        .follow_edges
                        .into_iter()
                        .map(|follow_edge| {
                            let original_from =
                                identity_map.get(&follow_edge.original_from).cloned();
                            let original_to = identity_map.get(&follow_edge.original_to).cloned();
                            let edge = FollowEdge {
                                follow_edge,
                                original_from,
                                original_to,
                            };
                            edge
                        })
                        .collect();
                    return Ok(Some(follow_edges));
                } else {
                    return Ok(Some(vec![]));
                }
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query follow_relation parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }

    pub async fn follow_relation(
        &self,
        client: &Client<HttpConnector>,
        hop: u16,
        follow_type: &str,
    ) -> Result<Option<SocialGraph>, Error> {
        let uri: http::Uri = format!(
            "{}/query/{}/follow_relation?g={}&follow_type={}&hop={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            self.graph_id.to_string(),
            follow_type.to_string(),
            hop,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!("query follow_relation Uri format Error | {}", _err))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!("query follow_relation ParamError Error {}", _err))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query follow_relation | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<RelationFollowResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query follow_relation error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r.results.and_then(|vec_res| vec_res.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query follow_relation parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }
}

pub struct IdentityGraphLoadFn {
    pub client: Client<HttpConnector>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BatchLoadIdentityGraphResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<Vec<IdentityGraph>>>,
}

#[async_trait::async_trait]
impl BatchFn<String, Option<IdentityGraph>> for IdentityGraphLoadFn {
    async fn load(&mut self, ids: &[String]) -> HashMap<String, Option<IdentityGraph>> {
        let records = query_identity_graph_by_ids(&self.client, ids.to_vec()).await;
        match records {
            Ok(records) => {
                let map: HashMap<String, Option<IdentityGraph>> = records
                    .into_iter()
                    .map(|graph| (graph.graph_id.to_string(), Some(graph)))
                    .collect();
                map
            }
            Err(_) => ids.iter().map(|k| (k.to_owned(), None)).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VertexIds {
    ids: Vec<String>,
}

pub async fn query_identity_graph_by_ids(
    client: &Client<HttpConnector>,
    ids: Vec<String>,
) -> Result<Vec<IdentityGraph>, Error> {
    let uri: http::Uri = format!(
        "{}/query/{}/query_identity_graph_by_ids",
        C.tdb.host,
        Graph::SocialGraph.to_string()
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let payload = VertexIds { ids };
    let json_params = serde_json::to_string(&payload).map_err(|err| Error::JSONParseError(err))?;
    let req = hyper::Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("Authorization", Graph::SocialGraph.token())
        .body(Body::from(json_params))
        .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "TigerGraph | Fail to query_identity_graph_by_ids: {:?}",
            err.to_string()
        ))
    })?;
    match parse_body::<IdentityGraphResponse>(&mut resp).await {
        Ok(r) => {
            if r.base.error {
                let err_message = format!(
                    "TigerGraph query_identity_graph_by_ids error | Code: {:?}, Message: {:?}",
                    r.base.code, r.base.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }

            let result = r.results.map_or(vec![], |res| res);
            Ok(result)
        }
        Err(err) => {
            let err_message = format!(
                "TigerGraph query_identity_graph_by_ids parse_body error: {:?}",
                err
            );
            error!(err_message);
            return Err(err);
        }
    }
}
