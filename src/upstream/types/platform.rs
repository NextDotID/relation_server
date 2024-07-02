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

    /// Bitcoin
    #[strum(serialize = "bitcoin")]
    #[serde(rename = "bitcoin")]
    #[graphql(name = "bitcoin")]
    Bitcoin,

    /// Ethereum wallet `0x[a-f0-9]{40}`
    #[strum(serialize = "ethereum", serialize = "eth")]
    #[serde(rename = "ethereum")]
    #[graphql(name = "ethereum")]
    Ethereum,

    /// Solana
    #[strum(serialize = "solana")]
    #[serde(rename = "solana")]
    #[graphql(name = "solana")]
    Solana,

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

    /// Facebook
    #[strum(serialize = "facebook")]
    #[serde(rename = "facebook")]
    #[graphql(name = "facebook")]
    Facebook,

    /// Instagram
    #[strum(serialize = "instagram")]
    #[serde(rename = "instagram")]
    #[graphql(name = "instagram")]
    Instagram,

    /// Mastodon maintained by Sujitech
    #[strum(serialize = "mstdnjp")]
    #[serde(rename = "mstdnjp")]
    #[graphql(name = "mstdnjp")]
    MstdnJP,

    /// Lobsters is a computing-focused community centered around link aggregation and discussion
    #[strum(serialize = "lobsters")]
    #[serde(rename = "lobsters")]
    #[graphql(name = "lobsters")]
    Lobsters,

    /// The Hacker News is the most trusted and popular cybersecurity publication for information security professionals seeking breaking news.
    #[strum(serialize = "hackernews")]
    #[serde(rename = "hackernews")]
    #[graphql(name = "hackernews")]
    HackerNews,

    /// ENS
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    #[graphql(name = "ens")]
    ENS,

    /// https://www.sns.id: Solana Name Service
    #[strum(serialize = "sns")]
    #[serde(rename = "sns")]
    #[graphql(name = "sns")]
    SNS,

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

    /// Genome
    #[strum(serialize = "genome")]
    #[serde(rename = "genome")]
    #[graphql(name = "genome")]
    Genome,

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

    /// Clusters
    /// https://docs.clusters.xyz/
    #[serde(rename = "clusters")]
    #[strum(serialize = "clusters")]
    #[graphql(name = "clusters")]
    Clusters,

    /// aptos
    #[serde(rename = "aptos")]
    #[strum(serialize = "aptos")]
    #[graphql(name = "aptos")]
    Aptos,

    /// near
    #[serde(rename = "near")]
    #[strum(serialize = "near")]
    #[graphql(name = "near")]
    Near,

    /// stacks
    #[serde(rename = "stacks")]
    #[strum(serialize = "stacks")]
    #[graphql(name = "stacks")]
    Stacks,

    /// xrpc
    #[serde(rename = "xrpc")]
    #[strum(serialize = "xrpc")]
    #[graphql(name = "xrpc")]
    Xrpc,

    /// cosmos
    #[serde(rename = "cosmos")]
    #[strum(serialize = "cosmos")]
    #[graphql(name = "cosmos")]
    Cosmos,

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
            Platform::ENS => DomainNameSystem::ENS,
            Platform::SNS => DomainNameSystem::SNS,
            Platform::Genome => DomainNameSystem::Genome,
            Platform::Clusters => DomainNameSystem::Clusters,
            _ => DomainNameSystem::Unknown,
        }
    }
}
