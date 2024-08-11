use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{Display, EnumIter, EnumString};

/// Status for Available Domain
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
    Hash,
    async_graphql::Enum,
)]
pub enum DomainStatus {
    /// Domain has been taken(already registered)
    #[strum(serialize = "taken")]
    #[serde(rename = "taken")]
    #[graphql(name = "taken")]
    Taken,

    /// Domain has been protected(Protected by domain platform to prevent preemptive registration)
    #[strum(serialize = "protected")]
    #[serde(rename = "protected")]
    #[graphql(name = "protected")]
    Protected,

    /// Domain available for registration
    #[default]
    #[strum(serialize = "available")]
    #[serde(rename = "available")]
    #[graphql(name = "available")]
    Available,
}
