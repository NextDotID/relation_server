use crate::{
    config::C,
    error::Error,
    tigergraph::{
        vertex::{FromWithParams, Identity, IdentityRecord, Vertex, VertexRecord},
        Attribute, BaseResponse, Graph, OpCode, Transfer,
    },
    upstream::{Chain, DataSource, Platform},
    util::{naive_now, parse_body},
};
use async_trait::async_trait;
use chrono::Duration;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::any::Any;
use std::collections::HashMap;
use tracing::error;

pub const VERTEX_NAME: &str = "IdentitiesGraph";

/// IdentitiesGraph
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IdentitiesGraph {
    /// UUID of this record
    pub id: String,
    /// microseconds are one-millionth of a second (1/1,000,000 seconds)
    pub updated_nanosecond: i64,
}

impl Default for IdentitiesGraph {
    fn default() -> Self {
        Self {
            id: String::from("fake_uuid_v4"),
            updated_nanosecond: Default::default(),
        }
    }
}

impl PartialEq for IdentitiesGraph {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// #[typetag::serde]
#[async_trait]
impl Vertex for IdentitiesGraph {
    fn primary_key(&self) -> String {
        self.id.clone()
    }

    fn vertex_type(&self) -> String {
        VERTEX_NAME.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdentitiesGraphRecord(pub VertexRecord<IdentitiesGraph>);

impl FromWithParams<IdentitiesGraph> for IdentitiesGraphRecord {
    fn from_with_params(v_type: String, v_id: String, attributes: IdentitiesGraph) -> Self {
        IdentitiesGraphRecord(VertexRecord {
            v_type,
            v_id,
            attributes,
        })
    }
}

impl From<VertexRecord<IdentitiesGraph>> for IdentitiesGraphRecord {
    fn from(record: VertexRecord<IdentitiesGraph>) -> Self {
        IdentitiesGraphRecord(record)
    }
}

impl std::ops::Deref for IdentitiesGraphRecord {
    type Target = VertexRecord<IdentitiesGraph>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for IdentitiesGraphRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for VertexRecord<IdentitiesGraph> {
    type Target = IdentitiesGraph;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for VertexRecord<IdentitiesGraph> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdentitiesGraphAttribute(HashMap<String, Attribute>);

// Implement `Transfer` trait for converting `IdentitiesGraph` into a `HashMap<String, Attribute>`.
impl Transfer for IdentitiesGraph {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        let mut attributes_map = HashMap::new();
        attributes_map.insert(
            "id".to_string(),
            Attribute {
                value: json!(self.id),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "updated_nanosecond".to_string(),
            Attribute {
                value: json!(self.updated_nanosecond),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), json!(self.id));
        map.insert(
            "updated_nanosecond".to_string(),
            json!(self.updated_nanosecond),
        );
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConnection {
    pub edge_type: String,
    pub data_source: DataSource,
    #[serde(rename = "source_v")]
    pub source: String,
    #[serde(rename = "target_v")]
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraph {
    pub graph_id: String,
    pub vertices: Vec<ExpandIdentityRecord>,
    pub edges: Vec<IdentityConnection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IdentityGraphResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<IdentityGraph>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SingleExpandIdentityResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<ExpandVerticesList>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExpandVerticesList {
    expand_vlist: Vec<ExpandIdentityRecord>,
}

impl IdentityGraph {
    pub async fn find_expand_identity(
        client: &Client<HttpConnector>,
        platform: &Platform,
        identity: &str,
    ) -> Result<Option<ExpandIdentityRecord>, Error> {
        let encoded_identity = urlencoding::encode(identity);
        let uri: http::Uri = format!(
            "{}/query/{}/find_expand_identity?platform={}&identity={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            platform.to_string(),
            encoded_identity,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!(
                "query find_expand_identity?platform={}&identity={} Uri format Error | {}",
                platform.to_string(),
                encoded_identity,
                _err
            ))
        })?;

        let req: http::Request<Body> = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!(
                    "query find_expand_identity ParamError Error {}",
                    _err
                ))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query find_expand_identity | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<SingleExpandIdentityResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query find_expand_identity error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r
                    .results
                    .and_then(|results| results.first().cloned())
                    .map(|result: ExpandVerticesList| result.expand_vlist)
                    .and_then(|res| res.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query find_identity_graph parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }
    pub async fn find_graph_by_platform_identity(
        client: &Client<HttpConnector>,
        platform: &Platform,
        identity: &str,
        reverse: Option<bool>,
    ) -> Result<Option<IdentityGraph>, Error> {
        // This reverse flag can be used as a filtering for Identity which type is domain system .
        // flag = 0, If `reverse=None` if omitted, there is no need to filter anything.
        // flag = 1, When `reverse=true`, just return `primary domain` related identities.
        // flag = 2, When `reverse=false`, Only `non-primary domain` will be returned, which is the inverse set of reverse=true.
        let flag = reverse.map_or(0, |r| match r {
            true => 1,
            false => 2,
        });
        let p = format!("{},{}", platform, identity);
        let encoded_p = urlencoding::encode(&p);
        let uri: http::Uri = format!(
            "{}/query/{}/find_identity_graph?p={}&reverse_flag={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            encoded_p,
            flag,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!(
                "query find_identity_graph?p={}&reverse_flag={} Uri format Error | {}",
                p, flag, _err
            ))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!(
                    "query find_identity_graph ParamError Error {}",
                    _err
                ))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query find_identity_graph | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<IdentityGraphResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query find_identity_graph error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r.results.and_then(|vec_res| vec_res.first().cloned());
                match result {
                    None => return Ok(None),
                    Some(result) => {
                        if result.graph_id == "" {
                            return Ok(None);
                        } else if result.edges.len() == 0 {
                            if result.vertices.len() > 1 {
                                return Ok(None); // If vertices=1, it's isolated vertex
                            }
                        } else {
                            // filter out dataSource == "basenames" edges
                            let filter_edges: Vec<IdentityConnection> = result
                                .edges
                                .clone()
                                .into_iter()
                                .filter(|e| e.data_source != DataSource::Basenames)
                                .collect();

                            if filter_edges.len() == 0 {
                                // only have basenames edges
                                let basenames_vertex: Vec<ExpandIdentityRecord> = result
                                    .vertices
                                    .clone()
                                    .into_iter()
                                    .filter(|v| v.record.platform == Platform::Ethereum)
                                    .collect();

                                if basenames_vertex.len() > 0 {
                                    let updated_at =
                                        basenames_vertex.first().cloned().unwrap().updated_at;
                                    let current_time = naive_now();
                                    let duration_since_update =
                                        current_time.signed_duration_since(updated_at);
                                    // Check if the difference is greater than 2 hours
                                    if duration_since_update > Duration::hours(2) {
                                        tracing::info!("Basenames refetching...");
                                        return Ok(None);
                                    }
                                }
                            }
                        }
                        return Ok(Some(result));
                    }
                }
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query find_identity_graph parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpandIdentityRecord {
    pub record: IdentityRecord,
    pub owner_address: Option<Vec<Address>>,
    pub resolve_address: Option<Vec<Address>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Address {
    pub chain: Chain,
    pub address: String,
}

impl std::ops::DerefMut for ExpandIdentityRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.record
    }
}

impl std::ops::Deref for ExpandIdentityRecord {
    type Target = IdentityRecord;
    fn deref(&self) -> &Self::Target {
        &self.record
    }
}

impl<'de> Deserialize<'de> for ExpandIdentityRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExpandIdentityRecordVisitor;
        impl<'de> Visitor<'de> for ExpandIdentityRecordVisitor {
            type Value = ExpandIdentityRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct ExpandIdentityRecord")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut v_type: Option<String> = None;
                let mut v_id: Option<String> = None;
                let mut attributes: Option<serde_json::Map<String, serde_json::Value>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "v_type" => v_type = Some(map.next_value()?),
                        "v_id" => v_id = Some(map.next_value()?),
                        "attributes" => attributes = Some(map.next_value()?),
                        _ => {}
                    }
                }

                let mut attributes =
                    attributes.ok_or_else(|| de::Error::missing_field("attributes"))?;

                let return_owner_address: Option<Vec<Address>> = attributes
                    .remove("@owner_address")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(de::Error::custom)?;

                let return_resolve_address: Option<Vec<Address>> = attributes
                    .remove("@resolve_address")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(de::Error::custom)?;

                let attributes: Identity =
                    serde_json::from_value(serde_json::Value::Object(attributes))
                        .map_err(de::Error::custom)?;

                let v_type = v_type.ok_or_else(|| de::Error::missing_field("v_type"))?;
                let v_id = v_id.ok_or_else(|| de::Error::missing_field("v_id"))?;

                let owner_address = match return_owner_address {
                    None => None,
                    Some(vec_address) => {
                        if vec_address.is_empty() {
                            None
                        } else {
                            Some(vec_address)
                        }
                    }
                };
                let resolve_address = match return_resolve_address {
                    None => None,
                    Some(vec_address) => {
                        if vec_address.is_empty() {
                            None
                        } else {
                            Some(vec_address)
                        }
                    }
                };
                let expand_identity = ExpandIdentityRecord {
                    record: IdentityRecord(VertexRecord {
                        v_type,
                        v_id,
                        attributes,
                    }),
                    owner_address,
                    resolve_address,
                };
                Ok(expand_identity)
            }
        }
        deserializer.deserialize_map(ExpandIdentityRecordVisitor)
    }
}
