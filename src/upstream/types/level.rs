use serde_repr::{Deserialize_repr, Serialize_repr};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    Display,
    EnumString,
    PartialEq,
    Eq,
    EnumIter,
    async_graphql::Enum,
    Default,
    Copy,
)]
#[repr(i32)]
pub enum ProofLevel {
    /// "ignore_if_exists" or "~"
    #[default]
    /// Low confidence
    Insecure = 1,
    /// Moderate-low confidence
    Cautious = 2,
    /// Moderate confidence
    Neutral = 3,
    /// Moderate-high confidence
    Confident = 4,
    /// High confidence
    VeryConfident = 5,
}
