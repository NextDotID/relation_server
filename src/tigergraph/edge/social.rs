use crate::{
    tigergraph::{
        edge::EdgeRecord,
        vertex::{ExpandIdentityRecord, IdentityRecord},
    },
    upstream::DataSource,
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationConnection {
    pub edge_type: String,
    pub tag: Option<String>,
    #[serde(rename = "source")]
    /// Data source (upstream) which provides this info.
    pub data_source: DataSource,
    /// The original follower/following elationship comes from specified identity
    pub original_from: String,
    /// The original follower/following elationship comes from specified identity
    pub original_to: String,
    /// When this HODL™ relation is fetched by us RelationService.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

impl Default for RelationConnection {
    fn default() -> Self {
        Self {
            tag: Some("".to_string()),
            edge_type: "".to_string(),
            data_source: DataSource::default(),
            original_from: "".to_string(),
            original_to: "".to_string(),
            updated_at: naive_now(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelationEdge(pub EdgeRecord<RelationConnection>);

impl From<EdgeRecord<RelationConnection>> for RelationEdge {
    fn from(record: EdgeRecord<RelationConnection>) -> Self {
        RelationEdge(record)
    }
}

impl std::ops::Deref for RelationEdge {
    type Target = EdgeRecord<RelationConnection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RelationEdge {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<RelationConnection> {
    type Target = RelationConnection;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<RelationConnection> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub relation: RelationEdge,
    pub original_from: Option<ExpandIdentityRecord>,
    pub original_to: Option<ExpandIdentityRecord>,
}
