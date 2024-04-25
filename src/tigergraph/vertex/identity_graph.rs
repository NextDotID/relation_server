use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{EdgeRecord, Relation, RelationConnection, RelationEdge, RelationResult},
        vertex::{Identity, IdentityRecord, VertexRecord},
        BaseResponse, Graph,
    },
    upstream::{Chain, DataSource, Platform},
    util::parse_body,
};
use chrono::NaiveDateTime;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tracing::error;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<RelationTopology>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationTopology {
    pub all_count: i32,
    pub edges: Vec<ExpandRelation>,
    pub vertices: Vec<IdentitiesGraphStatistic>,
    pub original_vertices: Vec<IdentityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitiesGraphStatistic {
    pub attributes: Statistic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistic {
    #[serde(rename = "id")]
    pub graph_id: String,
    #[serde(rename = "@range")]
    pub range: i32,
    #[serde(rename = "@degree")]
    pub degree: i32,
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
                            return Ok(None);
                        } else {
                            // filter out dataSource == "keybase" edges
                            let filter_edges: Vec<IdentityConnection> = result
                                .edges
                                .clone()
                                .into_iter()
                                .filter(|e| e.source != DataSource::Keybase.to_string())
                                .collect();
                            if filter_edges.len() == 0 {
                                return Ok(None);
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

    pub async fn follow(
        &self,
        client: &Client<HttpConnector>,
        hop: u16,
        data_source: Option<Vec<DataSource>>,
        limit: u16,
        offset: u16,
    ) -> Result<RelationResult, Error> {
        let uri: http::Uri;
        if data_source.is_none() || data_source.as_ref().unwrap().len() == 0 {
            uri = format!(
                "{}/query/{}/social_follows?g={}&hop={}&numPerPage={}&pageNum={}",
                C.tdb.host,
                Graph::SocialGraph.to_string(),
                self.graph_id.to_string(),
                hop,
                limit,
                offset
            )
            .parse()
            .map_err(|_err: InvalidUri| {
                Error::ParamError(format!("query social_follows Uri format Error {}", _err))
            })?;
        } else {
            let sources: Vec<String> = data_source
                .unwrap()
                .into_iter()
                .map(|field| format!("sources={}", field.to_string()))
                .collect();
            let combined = sources.join("&");
            uri = format!(
                "{}/query/{}/nfts?g={}&hop={}&{}&numPerPage={}&pageNum={}",
                C.tdb.host,
                Graph::SocialGraph.to_string(),
                self.graph_id.to_string(),
                hop,
                combined,
                limit,
                offset
            )
            .parse()
            .map_err(|_err: InvalidUri| {
                Error::ParamError(format!("query social_follows  Uri format Error {}", _err))
            })?;
        }

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!("query social_follows ParamError Error {}", _err))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query social_follows | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<RelationResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query follow_relation error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                if let Some(relations) = r.results.and_then(|res| res.first().cloned()) {
                    let identity_map: HashMap<String, IdentityRecord> = relations
                        .original_vertices
                        .into_iter()
                        .map(|record| (record.v_id.clone(), record))
                        .collect();

                    let statistic_map: HashMap<String, i32> = relations
                        .vertices
                        .into_iter()
                        .map(|record| (record.attributes.graph_id, record.attributes.degree))
                        .collect();

                    let relation_edges: Vec<Relation> = relations
                        .edges
                        .into_iter()
                        .map(|expand| {
                            let original_from = identity_map.get(&expand.original_from).cloned();
                            let original_to = identity_map.get(&expand.original_to).cloned();
                            let edge = Relation {
                                relation: expand.clone().record,
                                source_degree: statistic_map.get(&expand.record.from_id).cloned(),
                                target_degree: statistic_map.get(&expand.record.to_id).cloned(),
                                original_from,
                                original_to,
                            };
                            edge
                        })
                        .collect();

                    let result = RelationResult {
                        count: relations.all_count,
                        relation: relation_edges,
                    };
                    return Ok(result);
                } else {
                    return Ok(RelationResult {
                        count: 0,
                        relation: vec![],
                    });
                }
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query social_follows parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
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

#[derive(Debug, Clone, Serialize)]
pub struct ExpandIdentityRecord {
    pub record: IdentityRecord,
    pub owner_address: Option<Vec<Address>>,
    pub resolve_address: Option<Vec<Address>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpandRelation {
    pub record: RelationEdge,
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

impl std::ops::DerefMut for ExpandRelation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.record
    }
}

impl std::ops::Deref for ExpandRelation {
    type Target = RelationEdge;
    fn deref(&self) -> &Self::Target {
        &self.record
    }
}

impl<'de> Deserialize<'de> for ExpandRelation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExpandRelationRecordVisitor;
        impl<'de> Visitor<'de> for ExpandRelationRecordVisitor {
            type Value = ExpandRelation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct ExpandRelation")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let e_type: String = "Follow".to_string();
                let directed: bool = true;
                let from_type: String = "IdentitiesGraph".to_string();
                let to_type: String = "IdentitiesGraph".to_string();

                let mut source_v: Option<String> = None;
                let mut target_v: Option<String> = None;
                let mut original_from: Option<String> = None;
                let mut original_to: Option<String> = None;
                let mut data_source: Option<String> = None;
                let mut edge_type: Option<String> = None;
                let mut tag: Option<String> = None;
                let mut updated_at_str: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "source_v" => source_v = Some(map.next_value()?),
                        "target_v" => target_v = Some(map.next_value()?),
                        "original_from" => original_from = Some(map.next_value()?),
                        "original_to" => original_to = Some(map.next_value()?),
                        "data_source" => data_source = Some(map.next_value()?),
                        "edge_type" => edge_type = Some(map.next_value()?),
                        "tag" => tag = Some(map.next_value()?),
                        "updated_at" => updated_at_str = Some(map.next_value()?),
                        _ => {}
                    }
                }

                let from_id = source_v.ok_or_else(|| de::Error::missing_field("source_v"))?;
                let to_id = target_v.ok_or_else(|| de::Error::missing_field("target_v"))?;

                let edge_type = edge_type.ok_or_else(|| de::Error::missing_field("edge_type"))?;
                let tag = tag.ok_or_else(|| de::Error::missing_field("tag"))?;
                let data_source =
                    data_source.ok_or_else(|| de::Error::missing_field("data_source"))?;
                let original_from =
                    original_from.ok_or_else(|| de::Error::missing_field("original_from"))?;
                let original_to =
                    original_to.ok_or_else(|| de::Error::missing_field("original_to"))?;
                let updated_at_str =
                    updated_at_str.ok_or_else(|| de::Error::missing_field("updated_at"))?;

                let updated_at =
                    NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                        .map_err(serde::de::Error::custom)?;
                let attributes = RelationConnection {
                    edge_type,
                    tag: Some(tag),
                    data_source: DataSource::from_str(data_source.as_str())
                        .unwrap_or(DataSource::Unknown),
                    original_from,
                    original_to,
                    updated_at,
                };
                let edge_record = RelationEdge(EdgeRecord {
                    e_type,
                    directed,
                    from_id,
                    from_type,
                    to_id,
                    to_type,
                    discriminator: None,
                    attributes,
                });
                let expand_record = ExpandRelation {
                    record: edge_record,
                };
                Ok(expand_record)
            }
        }
        deserializer.deserialize_map(ExpandRelationRecordVisitor)
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
