use serde::{Serialize, Deserialize};
use strum_macros::{Display, EnumString, EnumIter};

/// Who collects all the data.
/// It works as a "data cleansing" or "proxy" between `Upstream`s and us.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Display,
    EnumString,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Copy,
    async_graphql::Enum,
)]
pub enum DataFetcher {
    /// This server
    #[strum(serialize = "relation_service")]
    #[serde(rename = "relation_service")]
    #[graphql(name = "relation_service")]
    #[default]
    RelationService,

    /// Aggregation service
    #[strum(serialize = "aggregation_service")]
    #[serde(rename = "aggregation_service")]
    #[graphql(name = "aggregation_service")]
    AggregationService,
}
