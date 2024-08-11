use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::AvailableDomain,
        vertex::{FromWithParams, Vertex, VertexRecord},
        Attribute, BaseResponse, Graph, OpCode, Transfer,
    },
    upstream::{Platform, EXT, EXTENSION},
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now, parse_body},
};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::any::Any;
use std::collections::HashMap;
use tracing::error;

pub const VERTEX_NAME: &str = "DomainCollection";

/// DomainCollection
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DomainCollection {
    /// root of domain name
    pub id: String,
    /// When it is updated (re-fetched) by us RelationService. Managed by us.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

impl Default for DomainCollection {
    fn default() -> Self {
        Self {
            id: Default::default(),
            updated_at: naive_now(),
        }
    }
}

impl PartialEq for DomainCollection {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[async_trait]
impl Vertex for DomainCollection {
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
pub struct DomainCollectionRecord(pub VertexRecord<DomainCollection>);

impl FromWithParams<DomainCollection> for DomainCollectionRecord {
    fn from_with_params(v_type: String, v_id: String, attributes: DomainCollection) -> Self {
        DomainCollectionRecord(VertexRecord {
            v_type,
            v_id,
            attributes,
        })
    }
}

impl From<VertexRecord<DomainCollection>> for DomainCollectionRecord {
    fn from(record: VertexRecord<DomainCollection>) -> Self {
        DomainCollectionRecord(record)
    }
}

impl std::ops::Deref for DomainCollectionRecord {
    type Target = VertexRecord<DomainCollection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DomainCollectionRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for VertexRecord<DomainCollection> {
    type Target = DomainCollection;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for VertexRecord<DomainCollection> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainCollectionAttribute(HashMap<String, Attribute>);

// Implement `Transfer` trait for converting `DomainCollection` into a `HashMap<String, Attribute>`.
impl Transfer for DomainCollection {
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
            "updated_at".to_string(),
            Attribute {
                value: json!(self.updated_at),
                op: Some(OpCode::Max),
            },
        );
        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("id".to_string(), json!(self.id));
        map.insert("updated_at".to_string(), json!(self.updated_at));
        map
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DomainAvailableSearchResultResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<DomainAvailableSearchResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainAvailableSearchResult {
    pub collection: Vec<DomainCollectionRecord>,
    pub domains: Vec<AvailableDomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainAvailableSearch {
    pub collection: DomainCollection,
    pub domains: Vec<AvailableDomain>,
}

impl DomainCollection {
    pub fn is_outdated(&self) -> bool {
        let current_time = naive_now();
        // Calculate the difference between the current time and updated_at
        let duration_since_update = current_time.signed_duration_since(self.updated_at);
        // Check if the difference is greater than 24 hours
        duration_since_update > Duration::hours(24)
    }

    pub async fn domain_available_search(
        client: &Client<HttpConnector>,
        name: &str,
    ) -> Result<Option<DomainAvailableSearch>, Error> {
        let encoded_name = urlencoding::encode(name);
        let uri: http::Uri = format!(
            "{}/query/{}/domain_available_search?id={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            encoded_name,
        )
        .parse()
        .map_err(|_err: InvalidUri| {
            Error::ParamError(format!(
                "query domain_available_search?id={} Uri format Error | {}",
                name, _err
            ))
        })?;

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| {
                Error::ParamError(format!(
                    "query domain_available_search?id={} ParamError Error {}",
                    name, _err
                ))
            })?;

        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query domain_available_search?id={} | Fail to request: {:?}",
                name,
                err.to_string()
            ))
        })?;

        match parse_body::<DomainAvailableSearchResultResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query domain_available_search?id={} error | Code: {:?}, Message: {:?}",
                        name, r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                let result = r.results.and_then(|vec_res| vec_res.first().cloned());
                match result {
                    None => return Ok(None),
                    Some(mut result) => {
                        if result.collection.is_empty() {
                            return Ok(None);
                        }
                        // Fill the domain available name list
                        let mut available_domains: Vec<AvailableDomain> = Vec::new();

                        for (domain_platform, exts) in EXTENSION.iter() {
                            let existing_tlds: Vec<String> = result
                                .domains
                                .iter()
                                .filter(|domain| domain.platform == *domain_platform)
                                .map(|domain| domain.tld.clone())
                                .collect();

                            if existing_tlds.is_empty() {
                                // If the domain_platform does not exist, add all tlds as available
                                if *domain_platform == Platform::Clusters {
                                    let cluster_parent = format!("{}{}", name, EXT::ClustersRoot);
                                    let cluster_child = format!("{}/{}", name, EXT::ClustersMain);
                                    available_domains.push(AvailableDomain {
                                        platform: domain_platform.clone(),
                                        name: cluster_parent,
                                        tld: EXT::ClustersRoot.to_string(),
                                        availability: true,
                                        status: "available".to_string(),
                                        expired_at: None,
                                    });
                                    available_domains.push(AvailableDomain {
                                        platform: domain_platform.clone(),
                                        name: cluster_child,
                                        tld: EXT::ClustersMain.to_string(),
                                        availability: true,
                                        status: "available".to_string(),
                                        expired_at: None,
                                    });
                                } else {
                                    for ext in exts {
                                        let domain_name = format!("{}.{}", name, ext);
                                        available_domains.push(AvailableDomain {
                                            platform: domain_platform.clone(),
                                            name: domain_name,
                                            tld: ext.to_string(),
                                            availability: true,
                                            status: "available".to_string(),
                                            expired_at: None,
                                        });
                                    }
                                }
                            } else {
                                // If the domain_platform exists, find missing tlds
                                for ext in exts {
                                    if !existing_tlds.contains(&ext.to_string()) {
                                        let domain_name = format!("{}.{}", name, ext);
                                        available_domains.push(AvailableDomain {
                                            platform: domain_platform.clone(),
                                            name: domain_name,
                                            tld: ext.to_string(),
                                            availability: true,
                                            status: "available".to_string(),
                                            expired_at: None,
                                        });
                                    }
                                }
                            }
                        }

                        // Update the result with the newly found available domains
                        result.domains.extend(available_domains);

                        match result.collection.first().cloned() {
                            None => return Ok(None),
                            Some(c) => {
                                return Ok(Some(DomainAvailableSearch {
                                    collection: c.attributes.clone(),
                                    domains: result.domains,
                                }))
                            }
                        }
                    }
                }
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query domain_available_search?id={} parse_body error: {:?}",
                    name, err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }
}
