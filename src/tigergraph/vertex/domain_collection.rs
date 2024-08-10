use crate::{
    tigergraph::{
        vertex::{FromWithParams, Vertex, VertexRecord},
        Attribute, OpCode, Transfer,
    },
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now},
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::any::Any;
use std::collections::HashMap;

pub const VERTEX_NAME: &str = "DomainCollection";

/// DomainCollection
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DomainCollection {
    /// label of domain name
    pub label: String,
    /// When it is updated (re-fetched) by us RelationService. Managed by us.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

impl Default for DomainCollection {
    fn default() -> Self {
        Self {
            label: Default::default(),
            updated_at: naive_now(),
        }
    }
}

impl PartialEq for DomainCollection {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
    }
}

#[async_trait]
impl Vertex for DomainCollection {
    fn primary_key(&self) -> String {
        self.label.clone()
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
            "label".to_string(),
            Attribute {
                value: json!(self.label),
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
        map.insert("label".to_string(), json!(self.label));
        map.insert("updated_at".to_string(), json!(self.updated_at));
        map
    }
}
