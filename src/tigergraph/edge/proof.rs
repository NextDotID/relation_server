use crate::{
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord, FromWithParams, Wrapper},
        upsert_graph,
        vertex::{Identity, Vertex, VertexRecord},
        Attribute, BaseResponse, EdgeWrapper, Edges, Graph, OpCode, Transfer, UpsertGraph,
    },
    upstream::{DataFetcher, DataSource, ProofLevel},
    util::{
        naive_datetime_from_string, naive_datetime_to_string, naive_now,
        option_naive_datetime_from_string, option_naive_datetime_to_string,
    },
};

use chrono::{Duration, NaiveDateTime};
use hyper::{client::HttpConnector, Client};
use serde::{Deserialize, Serialize};
use serde_json::value::{Map, Value};
use serde_json::{json, to_value};
use std::collections::HashMap;
use uuid::Uuid;

pub const EDGE_NAME: &str = "Proof_Forward";
pub const REVERSE_EDGE_NAME: &str = "Proof_Backward";
pub const IS_DIRECTED: bool = true;

/// Edge to connect two `Identity`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// UUID of this record. Generated by us to provide a better
    /// global-uniqueness for future P2P-network data exchange
    /// scenario.
    pub uuid: Uuid,
    /// Data source (upstream) which provides this connection info.
    pub source: DataSource,
    /// Level which provides this connection confidence level.
    pub level: ProofLevel,
    /// ID of this connection in upstream platform to locate (if any).
    pub record_id: Option<String>,
    /// When this connection is recorded in upstream platform (if platform gives such data).
    #[serde(deserialize_with = "option_naive_datetime_from_string")]
    #[serde(serialize_with = "option_naive_datetime_to_string")]
    pub created_at: Option<NaiveDateTime>,
    /// When this connection is fetched by us RelationService.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
    /// Who collects this data.
    /// It works as a "data cleansing" or "proxy" between `source`s and us.
    pub fetcher: DataFetcher,
}

impl Default for Proof {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            source: DataSource::default(),
            level: ProofLevel::default(),
            record_id: None,
            created_at: None,
            updated_at: naive_now(),
            fetcher: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofRecord(pub EdgeRecord<Proof>);

impl FromWithParams<Proof> for EdgeRecord<Proof> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: Proof,
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

impl From<EdgeRecord<Proof>> for ProofRecord {
    fn from(record: EdgeRecord<Proof>) -> Self {
        ProofRecord(record)
    }
}

impl std::ops::Deref for ProofRecord {
    type Target = EdgeRecord<Proof>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ProofRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<Proof> {
    type Target = Proof;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<Proof> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofAttribute(HashMap<String, Attribute>);

// Implement the `From` trait for converting `ProofRecord` into a `HashMap<String, Attr>`.
impl Transfer for ProofRecord {
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
        attributes_map.insert(
            "level".to_string(),
            Attribute {
                value: to_value(&self.attributes.level).unwrap(),
                op: None,
            },
        );

        if let Some(record_id) = self.attributes.record_id.clone() {
            attributes_map.insert(
                "record_id".to_string(),
                Attribute {
                    value: json!(record_id),
                    op: None,
                },
            );
        }

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
        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("uuid".to_string(), json!(self.uuid));
        map.insert("source".to_string(), json!(self.source));
        map.insert("level".to_string(), json!(self.level));
        map.insert(
            "record_id".to_string(),
            json!(self.record_id.clone().unwrap_or("".to_string())),
        );
        map.insert(
            "created_at".to_string(),
            self.created_at
                .map_or(json!("1970-01-01T00:00:00"), |created_at| json!(created_at)),
        );
        map.insert("updated_at".to_string(), json!(self.updated_at));
        map.insert("fetcher".to_string(), json!(self.fetcher));
        map
    }
}

impl Wrapper<ProofRecord, Identity, Identity> for Proof {
    fn wrapper(
        &self,
        from: &Identity,
        to: &Identity,
        name: &str,
    ) -> EdgeWrapper<ProofRecord, Identity, Identity> {
        let pb = EdgeRecord::from_with_params(
            name.to_string(),
            IS_DIRECTED,
            from.primary_key(),
            from.vertex_type(),
            to.primary_key(),
            to.vertex_type(),
            self.to_owned(),
        );
        EdgeWrapper {
            edge: ProofRecord(pb),
            source: from.to_owned(),
            target: to.to_owned(),
        }
    }
}

impl Proof {
    pub fn is_outdated(&self) -> bool {
        let outdated_in = Duration::try_days(1).unwrap();
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeResponse {
    #[serde(flatten)]
    base: BaseResponse,
    results: Option<Vec<ProofRecord>>,
}

#[async_trait::async_trait]
impl Edge<Identity, Identity, ProofRecord> for ProofRecord {
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
    ) -> Result<Option<ProofRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<Identity>,
        _to: &VertexRecord<Identity>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<ProofRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Identity,
        to: &Identity,
    ) -> Result<(), Error> {
        let pf = self.wrapper(from, to, EDGE_NAME);
        let edges = Edges(vec![pf]);
        let graph: UpsertGraph = edges.into();
        upsert_graph(client, &graph, Graph::SocialGraph).await?;
        Ok(())
    }

    /// Connect 2 vertex. For digraph and has reverse edge.
    async fn connect_reverse(
        &self,
        client: &Client<HttpConnector>,
        from: &Identity,
        to: &Identity,
    ) -> Result<(), Error> {
        let pf = self.wrapper(from, to, REVERSE_EDGE_NAME);
        let edges = Edges(vec![pf]);
        let graph: UpsertGraph = edges.into();
        upsert_graph(client, &graph, Graph::SocialGraph).await?;
        Ok(())
    }
}
