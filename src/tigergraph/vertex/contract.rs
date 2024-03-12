use crate::{
    config::C,
    error::Error,
    tigergraph::{
        upsert_graph,
        vertex::{FromWithParams, Vertex, VertexRecord},
        Attribute, BaseResponse, Graph, OpCode, Transfer, UpsertGraph, Vertices,
    },
    upstream::{Chain, ContractCategory},
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now, parse_body},
};

use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use dataloader::BatchFn;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::collections::HashMap;
use tracing::{error, trace};
use uuid::Uuid;

pub const VERTEX_NAME: &str = "Contracts";

/// Contract
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Contract {
    /// UUID of this record
    pub uuid: Uuid,
    /// What kind of Contract is it?
    pub category: ContractCategory,
    /// Contract address
    pub address: String,
    /// On which chain?
    pub chain: Chain,
    /// Token symbol
    pub symbol: Option<String>,
    /// When this data is fetched by RelationService.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            category: Default::default(),
            address: Default::default(),
            chain: Default::default(),
            symbol: Default::default(),
            updated_at: naive_now(),
        }
    }
}

impl PartialEq for Contract {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

// #[typetag::serde]
#[async_trait]
impl Vertex for Contract {
    fn primary_key(&self) -> String {
        format!("{},{}", self.chain, self.address)
    }

    fn vertex_type(&self) -> String {
        VERTEX_NAME.to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractRecord(pub VertexRecord<Contract>);

impl FromWithParams<Contract> for ContractRecord {
    fn from_with_params(v_type: String, v_id: String, attributes: Contract) -> Self {
        ContractRecord(VertexRecord {
            v_type,
            v_id,
            attributes,
        })
    }
}

impl From<VertexRecord<Contract>> for ContractRecord {
    fn from(record: VertexRecord<Contract>) -> Self {
        ContractRecord(record)
    }
}

impl std::ops::Deref for ContractRecord {
    type Target = VertexRecord<Contract>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ContractRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for VertexRecord<Contract> {
    type Target = Contract;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for VertexRecord<Contract> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractAttribute(HashMap<String, Attribute>);

// Implement `Transfer` trait for converting `Contract` into a `HashMap<String, Attribute>`.
impl Transfer for Contract {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        let mut attributes_map = HashMap::new();
        attributes_map.insert(
            "id".to_string(),
            Attribute {
                value: json!(self.primary_key()),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "uuid".to_string(),
            Attribute {
                value: json!(self.uuid),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "chain".to_string(),
            Attribute {
                value: json!(self.chain),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "address".to_string(),
            Attribute {
                value: json!(self.address),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "category".to_string(),
            Attribute {
                value: json!(self.category),
                op: None,
            },
        );
        if let Some(symbol) = self.symbol.clone() {
            attributes_map.insert(
                "symbol".to_string(),
                Attribute {
                    value: json!(symbol),
                    op: None,
                },
            );
        }
        attributes_map.insert(
            "updated_at".to_string(),
            Attribute {
                value: json!(self.updated_at),
                op: Some(OpCode::Max),
            },
        );

        attributes_map
    }

    fn to_json_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("id".to_string(), json!(self.primary_key()));
        map.insert("uuid".to_string(), json!(self.uuid));
        map.insert("chain".to_string(), json!(self.chain));
        map.insert("address".to_string(), json!(self.address));
        map.insert("category".to_string(), json!(self.category));
        map.insert(
            "symbol".to_string(),
            json!(self.symbol.clone().unwrap_or("".to_string())),
        );
        map.insert("updated_at".to_string(), json!(self.updated_at));
        Value::Object(map)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VertexResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<ContractRecord>>,
}

impl Contract {
    #[allow(dead_code)]
    fn uuid(&self) -> Option<uuid::Uuid> {
        Some(self.uuid)
    }

    /// Outdated in 1 hour
    #[allow(dead_code)]
    fn is_outdated(&self) -> bool {
        let outdated_in = Duration::hours(1);
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }

    /// Create or update a vertex.
    pub async fn create_or_update(&self, client: &Client<HttpConnector>) -> Result<(), Error> {
        let vertices = Vertices(vec![self.to_owned()]);
        let graph = UpsertGraph {
            vertices: vertices.into(),
            edges: None,
        };
        upsert_graph(client, &graph, Graph::IdentityGraph).await?;
        Ok(())
    }

    /// Find an Contract by UUID.
    #[allow(dead_code)]
    async fn find_by_uuid(
        client: &Client<HttpConnector>,
        uuid: Uuid,
    ) -> Result<Option<ContractRecord>, Error> {
        let uri: http::Uri = format!(
            "{}/graph/{}/vertices/{}?filter=uuid=%22{}%22",
            C.tdb.host,
            Graph::IdentityGraph.to_string(),
            VERTEX_NAME,
            uuid.to_string(),
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
                "query filter error | Fail to request: {:?}",
                err.to_string()
            ))
        })?;
        match parse_body::<VertexResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query filter error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                let result: Option<ContractRecord> = r
                    .results
                    .and_then(|results: Vec<ContractRecord>| results.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!("TigerGraph query filter parse_body error: {:?}", err);
                error!(err_message);
                return Err(err);
            }
        }
    }

    /// Find an Contract by `Chain` and `Address`
    pub async fn find_by_chain_address(
        client: &Client<HttpConnector>,
        chain: &Chain,
        address: &str,
    ) -> Result<Option<ContractRecord>, Error> {
        let uri: http::Uri = format!(
            "{}/graph/{}/vertices/{}?filter=chain=%22{}%22,address=%22{}%22",
            C.tdb.host,
            Graph::IdentityGraph.to_string(),
            VERTEX_NAME,
            chain.to_string(),
            address.to_string(),
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
                "query filter error | Fail to request: {:?}",
                err.to_string()
            ))
        })?;
        match parse_body::<VertexResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query filter error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                let result: Option<ContractRecord> = r
                    .results
                    .and_then(|results: Vec<ContractRecord>| results.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!("TigerGraph query filter parse_body error: {:?}", err);
                error!(err_message);
                return Err(err);
            }
        }
    }
}

pub struct ContractLoadFn {
    pub client: Client<HttpConnector>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VertexIds {
    ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VertexIdsResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<VertexIdsResult>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VertexIdsResult {
    vertices: Vec<ContractRecord>,
}

#[async_trait::async_trait]
impl BatchFn<String, Option<ContractRecord>> for ContractLoadFn {
    async fn load(&mut self, ids: &[String]) -> HashMap<String, Option<ContractRecord>> {
        trace!(ids = ids.len(), "Loading Contract id");
        let records = get_contracts_by_ids(&self.client, ids.to_vec()).await;
        match records {
            Ok(records) => records,
            // HOLD ON: Not sure if `Err` need to return
            Err(_) => ids.iter().map(|k| (k.to_owned(), None)).collect(),
        }
    }
}

async fn get_contracts_by_ids(
    client: &Client<HttpConnector>,
    ids: Vec<String>,
) -> Result<HashMap<String, Option<ContractRecord>>, Error> {
    let uri: http::Uri = format!(
        "{}/query/{}/contracts_by_ids",
        C.tdb.host,
        Graph::IdentityGraph.to_string()
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let payload = VertexIds { ids };
    let json_params = serde_json::to_string(&payload).map_err(|err| Error::JSONParseError(err))?;
    let req = hyper::Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("Authorization", Graph::IdentityGraph.token())
        .body(Body::from(json_params))
        .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
    let mut resp = client.request(req).await.map_err(|err| {
        Error::ManualHttpClientError(format!(
            "TigerGraph | Fail to request contracts_by_ids graph: {:?}",
            err.to_string()
        ))
    })?;
    match parse_body::<VertexIdsResponse>(&mut resp).await {
        Ok(r) => {
            if r.base.error {
                let err_message = format!(
                    "TigerGraph contracts_by_ids error | Code: {:?}, Message: {:?}",
                    r.base.code, r.base.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }

            let result = r
                .results
                .and_then(|results| results.first().cloned())
                .map_or(vec![], |res| res.vertices)
                .into_iter()
                .map(|content| (content.v_id.clone(), Some(content)))
                .collect();
            Ok(result)
        }
        Err(err) => {
            let err_message = format!("TigerGraph contracts_by_ids parse_body error: {:?}", err);
            error!(err_message);
            return Err(err);
        }
    }
}
