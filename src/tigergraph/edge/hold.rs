use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord, EdgeWrapper, FromWithParams, Wrapper},
        upsert_graph,
        vertex::{contract::VERTEX_NAME as CONTRACTS, Contract, Identity, Vertex, VertexRecord},
        Attribute, BaseResponse, Edges, Graph, OpCode, Transfer, UpsertGraph,
    },
    upstream::{Chain, DataFetcher, DataSource},
    util::{
        naive_datetime_from_string, naive_datetime_to_string, naive_now,
        option_naive_datetime_from_string, option_naive_datetime_to_string, parse_body,
    },
};

use async_graphql::SimpleObject;
use chrono::{Duration, NaiveDateTime};
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

pub const HOLD_IDENTITY: &str = "Hold_Identity";
pub const HOLD_CONTRACT: &str = "Hold_Contract";
pub const IS_DIRECTED: bool = true;

/// HODL™
#[derive(SimpleObject, Clone, Deserialize, Serialize, Debug)]
pub struct Hold {
    /// UUID of this record.
    pub uuid: Uuid,
    /// Data source (upstream) which provides this info.
    /// Theoretically, Contract info should only be fetched by chain's RPC server,
    /// but in practice, we still rely on third-party cache / snapshot service.
    pub source: DataSource,
    /// Transaction info of this connection.
    /// i.e. in which `tx` the Contract is transferred / minted.
    /// In most case, it is a `"0xVERY_LONG_HEXSTRING"`.
    /// It happens that this info is not provided by `source`, so we treat it as `Option<>`.
    pub transaction: Option<String>,
    /// NFT_ID in contract / ENS domain / anything can be used as an unique ID to specify the held object.
    /// It must be one here.
    /// Tips: NFT_ID of ENS is a hash of domain. So domain can be used as NFT_ID.
    pub id: String,
    /// When the transaction happened. May not be provided by upstream.
    #[serde(deserialize_with = "option_naive_datetime_from_string")]
    #[serde(serialize_with = "option_naive_datetime_to_string")]
    pub created_at: Option<NaiveDateTime>,
    /// When this HODL™ relation is fetched by us RelationService.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
    /// Who collects this data.
    /// It works as a "data cleansing" or "proxy" between `source`s and us.
    pub fetcher: DataFetcher,
    #[serde(deserialize_with = "option_naive_datetime_from_string")]
    #[serde(serialize_with = "option_naive_datetime_to_string")]
    pub expired_at: Option<NaiveDateTime>,
}

impl Default for Hold {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            source: DataSource::default(),
            transaction: None,
            id: "".to_string(),
            created_at: None,
            updated_at: naive_now(),
            fetcher: Default::default(),
            expired_at: None,
        }
    }
}

impl PartialEq for Hold {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HoldRecord(pub EdgeRecord<Hold>);

impl FromWithParams<Hold> for EdgeRecord<Hold> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: Hold,
    ) -> Self {
        EdgeRecord {
            e_type,
            directed,
            from_id,
            from_type,
            to_id,
            to_type,
            discriminator: None,
            attributes,
        }
    }
}

impl From<EdgeRecord<Hold>> for HoldRecord {
    fn from(record: EdgeRecord<Hold>) -> Self {
        HoldRecord(record)
    }
}

impl std::ops::Deref for HoldRecord {
    type Target = EdgeRecord<Hold>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HoldRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<Hold> {
    type Target = Hold;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<Hold> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HoldAttribute(HashMap<String, Attribute>);

// Implement the `From` trait for converting `HoldRecord` into a `HashMap<String, Attr>`.
impl Transfer for HoldRecord {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        let mut attributes_map = HashMap::new();
        attributes_map.insert(
            "uuid".to_string(),
            Attribute {
                value: json!(self.attributes.uuid.to_string()),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "source".to_string(),
            Attribute {
                value: json!(self.attributes.source.to_string()),
                op: None,
            },
        );
        if let Some(transaction) = self.attributes.transaction.clone() {
            attributes_map.insert(
                "transaction".to_string(),
                Attribute {
                    value: json!(transaction),
                    op: None,
                },
            );
        }
        attributes_map.insert(
            "id".to_string(),
            Attribute {
                value: json!(self.attributes.id),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        if let Some(created_at) = self.attributes.created_at {
            attributes_map.insert(
                "created_at".to_string(),
                Attribute {
                    value: json!(created_at),
                    op: Some(OpCode::IgnoreIfExists),
                },
            );
        }
        attributes_map.insert(
            "updated_at".to_string(),
            Attribute {
                value: json!(self.attributes.updated_at),
                op: Some(OpCode::Max),
            },
        );
        attributes_map.insert(
            "fetcher".to_string(),
            Attribute {
                value: json!(self.attributes.fetcher.to_string()),
                op: None,
            },
        );

        if let Some(expired_at) = self.expired_at {
            attributes_map.insert(
                "expired_at".to_string(),
                Attribute {
                    value: json!(expired_at),
                    op: Some(OpCode::Max),
                },
            );
        } else {
            attributes_map.insert(
                "expired_at".to_string(),
                Attribute {
                    value: json!("1970-01-01T00:00:00"), // default value
                    op: Some(OpCode::Max),
                },
            );
        }
        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("uuid".to_string(), json!(self.uuid));
        map.insert("source".to_string(), json!(self.source));
        map.insert(
            "transaction".to_string(),
            json!(self.transaction.clone().unwrap_or("".to_string())),
        );
        map.insert("id".to_string(), json!(self.id));
        map.insert(
            "created_at".to_string(),
            self.created_at
                .map_or(json!("1970-01-01T00:00:00"), |created_at| json!(created_at)),
        );
        map.insert("updated_at".to_string(), json!(self.updated_at));
        map.insert("fetcher".to_string(), json!(self.fetcher));
        map.insert(
            "expired_at".to_string(),
            self.expired_at
                .map_or(json!("1970-01-01T00:00:00"), |expired_at| json!(expired_at)),
        );
        map
    }
}

impl Wrapper<HoldRecord, Identity, Identity> for Hold {
    fn wrapper(
        &self,
        from: &Identity,
        to: &Identity,
        name: &str,
    ) -> EdgeWrapper<HoldRecord, Identity, Identity> {
        let hold = EdgeRecord::from_with_params(
            name.to_string(),
            IS_DIRECTED,
            from.primary_key(),
            from.vertex_type(),
            to.primary_key(),
            to.vertex_type(),
            self.to_owned(),
        );
        EdgeWrapper {
            edge: HoldRecord(hold),
            source: from.to_owned(),
            target: to.to_owned(),
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Identity, HoldRecord> for HoldRecord {
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
    ) -> Result<Option<HoldRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<Identity>,
        _to: &VertexRecord<Identity>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<HoldRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Identity,
        to: &Identity,
    ) -> Result<(), Error> {
        let hold_identity = self.wrapper(from, to, HOLD_IDENTITY);
        let edges = Edges(vec![hold_identity]);
        let graph: UpsertGraph = edges.into();
        upsert_graph(client, &graph, Graph::SocialGraph).await?;
        Ok(())
    }

    /// notice this function is deprecated
    async fn connect_reverse(
        &self,
        _client: &Client<HttpConnector>,
        _from: &Identity,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }
}

impl Wrapper<HoldRecord, Identity, Contract> for Hold {
    fn wrapper(
        &self,
        from: &Identity,
        to: &Contract,
        name: &str,
    ) -> EdgeWrapper<HoldRecord, Identity, Contract> {
        let hold = EdgeRecord::from_with_params(
            name.to_string(),
            IS_DIRECTED,
            from.primary_key(),
            from.vertex_type(),
            to.primary_key(),
            to.vertex_type(),
            self.to_owned(),
        );
        EdgeWrapper {
            edge: HoldRecord(hold),
            source: from.to_owned(),
            target: to.to_owned(),
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Contract, HoldRecord> for HoldRecord {
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
    ) -> Result<Option<HoldRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<Identity>,
        _to: &VertexRecord<Contract>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<HoldRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Identity,
        to: &Contract,
    ) -> Result<(), Error> {
        let hold_contract = self.wrapper(from, to, HOLD_CONTRACT);
        let edges = Edges(vec![hold_contract]);
        let graph: UpsertGraph = edges.into();
        upsert_graph(client, &graph, Graph::SocialGraph).await?;
        Ok(())
    }

    /// notice this function is deprecated
    async fn connect_reverse(
        &self,
        _client: &Client<HttpConnector>,
        _from: &Identity,
        _to: &Contract,
    ) -> Result<(), Error> {
        todo!()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<HoldRecord>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NftHolderResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<NftHolder>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NftHolder {
    holds: Vec<HoldRecord>,
}

impl Hold {
    pub fn is_outdated(&self) -> bool {
        let outdated_in = Duration::try_hours(8).unwrap();
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }

    /// find `EdgeRecord` by source_id and target_id
    pub async fn find_by_from_to<T: Vertex + std::marker::Sync>(
        client: &Client<HttpConnector>,
        from: &VertexRecord<Identity>,
        to: &VertexRecord<T>,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<HoldRecord>>, Error> {
        // http://host/graph/SocialGraph/edges/
        // <source_vtype>/<source_vid>/<e_type>/<target_vtype>/<target_vid>?filter=a="b"
        let e_type = if to.v_type == CONTRACTS {
            HOLD_CONTRACT
        } else {
            HOLD_IDENTITY
        };

        let uri: http::Uri;
        match filters {
            Some(filters) => {
                let filter_string = filters
                    .into_iter()
                    .map(|filter| format!("{}=%22{}%22", filter.0, filter.1))
                    .collect::<Vec<String>>()
                    .join(",");
                tracing::debug!("{}", filter_string);
                uri = format!(
                    "{}/graph/{}/edges/{}/{}/{}/{}/{}?filter={}",
                    C.tdb.host,
                    Graph::SocialGraph.to_string(),
                    from.v_type,
                    from.v_id,
                    e_type,
                    to.v_type,
                    to.v_id,
                    filter_string,
                )
                .parse()
                .map_err(|_err: InvalidUri| {
                    Error::ParamError(format!("Uri format Error {}", _err))
                })?;
            }
            None => {
                uri = format!(
                    "{}/graph/{}/edges/{}/{}/{}/{}/{}",
                    C.tdb.host,
                    Graph::SocialGraph.to_string(),
                    from.v_type,
                    from.v_id,
                    e_type,
                    to.v_type,
                    to.v_id,
                )
                .parse()
                .map_err(|_err: InvalidUri| {
                    Error::ParamError(format!("Uri format Error {}", _err))
                })?;
            }
        }

        let req = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", Graph::SocialGraph.token())
            .body(Body::empty())
            .map_err(|_err| Error::ParamError(format!("ParamError Error {}", _err)))?;
        let mut resp = client.request(req).await.map_err(|err| {
            Error::ManualHttpClientError(format!(
                "query find_by_from_to | Fail to request: {:?}",
                err.to_string()
            ))
        })?;

        match parse_body::<EdgeResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query find_by_from_to error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }
                Ok(r.results)
            }
            Err(err) => {
                let err_message = format!(
                    "TigerGraph query find_by_from_to parse_body error: {:?}",
                    err
                );
                error!(err_message);
                return Err(err);
            }
        }
    }

    /// Find a hold record by Chain, NFT_ID and NFT Address.
    /// merge these 2 queries into one.
    pub async fn find_by_id_chain_address(
        client: &Client<HttpConnector>,
        id: &str,
        chain: &Chain,
        address: &str,
    ) -> Result<Option<HoldRecord>, Error> {
        let encoded_id = urlencoding::encode(id);
        let uri: http::Uri = format!(
            "{}/query/{}/hold_nft?id={}&chain={}&address={}",
            C.tdb.host,
            Graph::SocialGraph.to_string(),
            encoded_id,
            chain.to_string(),
            address.to_string(),
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
                "query holder | Fail to request: {:?}",
                err.to_string()
            ))
        })?;
        match parse_body::<NftHolderResponse>(&mut resp).await {
            Ok(r) => {
                if r.base.error {
                    let err_message = format!(
                        "TigerGraph query holder error | Code: {:?}, Message: {:?}",
                        r.base.code, r.base.message
                    );
                    error!(err_message);
                    return Err(Error::General(err_message, resp.status()));
                }

                let result = r
                    .results
                    .and_then(|vec_holders| vec_holders.first().cloned())
                    .map(|holders| holders.holds)
                    .and_then(|res| res.first().cloned());
                Ok(result)
            }
            Err(err) => {
                let err_message = format!("TigerGraph query holder parse_body error: {:?}", err);
                error!(err_message);
                return Err(err);
            }
        }
    }
}
