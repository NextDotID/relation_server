use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord},
        vertex::{Identity, Vertex, VertexRecord},
        BaseResponse, Graph,
    },
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now, parse_body},
};

use chrono::NaiveDateTime;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

pub const EDGE_NAME: &str = "Relation_Unique_TX";
pub const IS_DIRECTED: bool = true;

/// Edge to connect two `Identity`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationUniqueTX {
    pub count: u32,
    pub sum: u32,
    pub max: u32,
    pub min: u32,
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationUniqueTXRecord(pub EdgeRecord<RelationUniqueTX>);

impl Default for RelationUniqueTX {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0,
            max: 0,
            min: 0,
            updated_at: naive_now(),
        }
    }
}

impl From<EdgeRecord<RelationUniqueTX>> for RelationUniqueTXRecord {
    fn from(record: EdgeRecord<RelationUniqueTX>) -> Self {
        RelationUniqueTXRecord(record)
    }
}

impl std::ops::Deref for RelationUniqueTXRecord {
    type Target = EdgeRecord<RelationUniqueTX>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RelationUniqueTXRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<RelationUniqueTX> {
    type Target = RelationUniqueTX;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<RelationUniqueTX> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Identity, RelationUniqueTXRecord> for RelationUniqueTXRecord {
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
    ) -> Result<Option<RelationUniqueTXRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<Identity>,
        _to: &VertexRecord<Identity>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<RelationUniqueTXRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        _client: &Client<HttpConnector>,
        _from: &Identity,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Connect 2 vertex. For digraph and has reverse edge.
    async fn connect_reverse(
        &self,
        _client: &Client<HttpConnector>,
        _from: &Identity,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<RelationUnions>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationUnions {
    relations: Vec<RelationUniqueTXRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpandResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<RelationUnions>>,
}

impl RelationUniqueTX {
    /// Find relatoin between source and target
    pub async fn relation(
        client: &Client<HttpConnector>,
        source: &Identity,
        target: &Identity,
        depth: u16,
    ) -> Result<Vec<RelationUniqueTXRecord>, Error> {
        // 1. expand source & target identity
        // 2. find all paths between source and target
        // source and target must contain ethereum address to find unique tx
        let uri: http::Uri = format!(
            "{}/query/{}/relation_single_pair?v_source={}&target_v={}&depth={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            source.primary_key(),
            target.primary_key(),
            depth,
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query relation | Fail to request: {:?}",
                err.to_string()
            ))
        })?;
        match parse_body::<RelationResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query relation error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r
                    .results
                    .and_then(|vec_unions| vec_unions.first().cloned())
                    .map_or(vec![], |union| union.relations);
                Ok(result)
            }
            Err(err) => {
                let err_message = format!("TigerGraph query relation parse_body error: {:?}", err);
                error!(err_message);
                return Err(err);
            }
        }
    }

    pub async fn expand(
        client: &Client<HttpConnector>,
        source: &Identity,
        depth: u16,
    ) -> Result<Vec<RelationUniqueTXRecord>, Error> {
        let uri: http::Uri = format!(
            "{}/query/{}/expand?p={}&depth={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            source.primary_key(),
            depth,
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query relation | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<ExpandResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query relation error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r
                    .results
                    .and_then(|vec_unions| vec_unions.first().cloned())
                    .map_or(vec![], |union| union.relations);
                Ok(result)
            }
            Err(err) => {
                let err_message = format!("TigerGraph query relation parse_body error: {:?}", err);
                error!(err_message);
                return Err(err);
            }
        }
    }
}
