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
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::error;
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VertexResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<ContractRecord>>,
}

impl Contract {
    fn uuid(&self) -> Option<uuid::Uuid> {
        Some(self.uuid)
    }

    /// Outdated in 1 hour
    fn is_outdated(&self) -> bool {
        let outdated_in = Duration::hours(1);
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }

    /// Do create / update
    pub async fn create_or_update(&self, client: &Client<HttpConnector>) -> Result<(), Error> {
        let vertices = Vertices(vec![self.to_owned()]);
        let graph = UpsertGraph {
            vertices: vertices.into(),
            edges: None,
        };
        let json_raw = serde_json::to_string(&graph).map_err(|err| Error::JSONParseError(err))?;
        println!("create_or_update {}", json_raw);
        upsert_graph(client, &graph, Graph::IdentityGraph).await?;
        Ok(())
    }

    /// Find an Contract by UUID.
    async fn find_by_uuid(
        client: &Client<HttpConnector>,
        uuid: Uuid,
    ) -> Result<Option<ContractRecord>, Error> {
        let uri: http::Uri = format!(
            "{}/graph/{}/vertices/{}?filter=uuid=%22{}%22",
            C.tdb.host,
            Graph::AssetGraph.to_string(),
            VERTEX_NAME,
            uuid.to_string(),
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::AssetGraph.token())
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
            Graph::AssetGraph.to_string(),
            VERTEX_NAME,
            chain.to_string(),
            address.to_string(),
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::AssetGraph.token())
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
