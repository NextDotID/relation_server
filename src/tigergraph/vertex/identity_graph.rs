use crate::{
    config::C,
    error::Error,
    tigergraph::{edge::SocialGraph, vertex::IdentityRecord, BaseResponse, Graph},
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
            Ok(records) => records,
            Err(_) => ids.iter().map(|k| (k.to_owned(), None)).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VertexIds {
    ids: Vec<String>,
}

async fn query_identity_graph_by_ids(
    client: &Client<HttpConnector>,
    ids: Vec<String>,
) -> Result<HashMap<String, Option<IdentityGraph>>, Error> {
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

            // let result = r.results.and_then(|results| results.first().cloned()).map_or(default, f)
            let result = r
                .results
                .map_or(vec![], |res| res)
                .into_iter()
                .map(|graph| (graph.graph_id.to_string(), Some(graph)))
                .collect();
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
