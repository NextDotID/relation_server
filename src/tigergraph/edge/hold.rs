use crate::{
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord, EdgeWrapper, FromWithParams, Wrapper},
        vertex::{Identity, Vertex},
        Attribute, OpCode, Transfer,
    },
    upstream::{DataFetcher, DataSource},
    util::{
        naive_datetime_from_string, naive_datetime_to_string, naive_now,
        option_naive_datetime_from_string, option_naive_datetime_to_string, parse_body,
    },
};

use async_graphql::{ObjectType, SimpleObject};
use chrono::{Duration, NaiveDateTime};
use hyper::{client::HttpConnector, Body, Client, Request};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_value};
use std::collections::HashMap;
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
        attributes_map
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

    /// Connect 2 vertex.
    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Identity,
        to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }
}

// impl From<EdgeWrapper<HoldRecord, Identity, Identity>> for HoldIdentity {
//     fn from(record: EdgeWrapper<HoldRecord, Identity, Identity>) -> Self {
//         HoldIdentity(record)
//     }
// }

// impl std::ops::Deref for HoldIdentity {
//     type Target = EdgeWrapper<HoldRecord, Identity, Identity>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl std::ops::DerefMut for HoldIdentity {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct HoldIdentity(EdgeWrapper<HoldRecord, Identity, Identity>);

// impl HoldIdentity {
//     pub fn new(
//         hold: &impl Wrapper<HoldRecord, Identity, Identity>,
//         from: &Identity,
//         to: &Identity,
//         name: &str,
//     ) -> Self {
//         HoldIdentity(hold.wrapper(from, to, name))
//     }
// }
