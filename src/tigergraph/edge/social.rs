use crate::{
    tigergraph::edge::EdgeRecord,
    upstream::DataSource,
    util::{naive_datetime_from_string, naive_datetime_to_string, naive_now},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Follow {
    #[serde(rename = "source")]
    /// Data source (upstream) which provides this info.
    pub source: DataSource,
    /// The original follower/following elationship comes from specified identity
    pub original_from: String,
    /// The original follower/following elationship comes from specified identity
    pub original_to: String,
    /// When this HODL™ relation is fetched by us RelationService.
    #[serde(deserialize_with = "naive_datetime_from_string")]
    #[serde(serialize_with = "naive_datetime_to_string")]
    pub updated_at: NaiveDateTime,
}

impl Default for Follow {
    fn default() -> Self {
        Self {
            source: DataSource::default(),
            original_from: "".to_string(),
            original_to: "".to_string(),
            updated_at: naive_now(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SocialFollow(pub EdgeRecord<Follow>);

impl From<EdgeRecord<Follow>> for SocialFollow {
    fn from(record: EdgeRecord<Follow>) -> Self {
        SocialFollow(record)
    }
}

impl std::ops::Deref for SocialFollow {
    type Target = EdgeRecord<Follow>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SocialFollow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<Follow> {
    type Target = Follow;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<Follow> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraph {
    pub list: Option<Vec<Uuid>>,
    pub topology: Option<Vec<SocialFollow>>,
}
