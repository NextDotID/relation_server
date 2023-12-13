use crate::upstream::DomainNameSystem;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// All identity platform.
/// TODO: move this definition into `graph/vertex/identity`, since it is not specific to upstream.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    EnumString,
    Clone,
    Copy,
    Display,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Hash,
    async_graphql::Enum,
)]
pub enum Platform {
    /// Twitter
    #[strum(serialize = "twitter")]
    #[serde(rename = "twitter")]
    #[graphql(name = "twitter")]
    Twitter,

    /// Ethereum wallet `0x[a-f0-9]{40}`
    #[strum(serialize = "ethereum", serialize = "eth")]
    #[serde(rename = "ethereum")]
    #[graphql(name = "ethereum")]
    Ethereum,

    /// NextID
    #[strum(serialize = "nextid")]
    #[serde(rename = "nextid")]
    #[graphql(name = "nextid")]
    NextID,

    /// Keybase
    #[strum(serialize = "keybase")]
    #[serde(rename = "keybase")]
    #[graphql(name = "keybase")]
    Keybase,

    /// Github
    #[strum(serialize = "github")]
    #[serde(rename = "github")]
    #[graphql(name = "github")]
    Github,

    /// Reddit
    #[strum(serialize = "reddit")]
    #[serde(rename = "reddit")]
    #[graphql(name = "reddit")]
    Reddit,

    /// Lens
    #[strum(serialize = "Lens", serialize = "lens")]
    #[serde(rename = "lens")]
    #[graphql(name = "lens")]
    Lens,

    /// .bit
    #[strum(serialize = "dotbit")]
    #[serde(rename = "dotbit")]
    #[graphql(name = "dotbit")]
    Dotbit,

    /// DNS
    #[strum(serialize = "dns")]
    #[serde(rename = "dns")]
    #[graphql(name = "dns")]
    DNS,

    /// Minds
    #[strum(serialize = "minds")]
    #[serde(rename = "minds")]
    #[graphql(name = "minds")]
    Minds,

    /// UnstoppableDomains
    #[strum(serialize = "unstoppabledomains")]
    #[serde(rename = "unstoppabledomains")]
    #[graphql(name = "unstoppabledomains")]
    UnstoppableDomains,

    /// Farcaster
    #[strum(serialize = "farcaster")]
    #[serde(rename = "farcaster")]
    #[graphql(name = "farcaster")]
    Farcaster,

    /// SpaceId
    #[strum(serialize = "space_id")]
    #[serde(rename = "space_id")]
    #[graphql(name = "space_id")]
    SpaceId,

    /// Crossbell
    #[strum(serialize = "crossbell")]
    #[serde(rename = "crossbell")]
    #[graphql(name = "crossbell")]
    Crossbell,

    /// CKB
    #[strum(serialize = "ckb")]
    #[serde(rename = "ckb")]
    #[graphql(name = "ckb")]
    CKB,

    /// Tron
    #[strum(serialize = "tron")]
    #[serde(rename = "tron")]
    #[graphql(name = "tron")]
    Tron,

    /// doge
    #[strum(serialize = "doge")]
    #[serde(rename = "doge")]
    #[graphql(name = "doge")]
    Doge,

    /// BNB Smart Chain (BSC)
    #[strum(serialize = "bsc")]
    #[serde(rename = "bsc")]
    #[graphql(name = "bsc")]
    BNBSmartChain,

    /// Polygon
    #[serde(rename = "polygon")]
    #[strum(serialize = "polygon")]
    #[graphql(name = "polygon")]
    Polygon,

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    #[default]
    Unknown,
}

impl From<Platform> for DomainNameSystem {
    fn from(platform: Platform) -> Self {
        match platform {
            Platform::Dotbit => DomainNameSystem::DotBit,
            Platform::UnstoppableDomains => DomainNameSystem::UnstoppableDomains,
            Platform::Lens => DomainNameSystem::Lens,
            Platform::SpaceId => DomainNameSystem::SpaceId,
            Platform::Crossbell => DomainNameSystem::SpaceId,
            _ => DomainNameSystem::Unknown,
        }
    }
}
