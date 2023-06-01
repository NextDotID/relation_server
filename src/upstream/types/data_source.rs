use crate::error::Error;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// All data respource platform.
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
pub enum DataSource {
    /// https://github.com/Uniswap/sybil-list/blob/master/verified.json
    #[strum(serialize = "sybil")]
    #[serde(rename = "sybil")]
    #[graphql(name = "sybil")]
    SybilList,

    /// https://keybase.io/docs/api/1.0/call/user/lookup
    #[strum(serialize = "keybase")]
    #[serde(rename = "keybase")]
    #[graphql(name = "keybase")]
    Keybase,

    /// https://docs.next.id/docs/proof-service/api
    #[strum(serialize = "nextid")]
    #[serde(rename = "nextid")]
    #[graphql(name = "nextid")]
    NextID, // = "nextID",

    /// https://rss3.io/network/api.html
    #[strum(serialize = "rss3")]
    #[serde(rename = "rss3")]
    #[graphql(name = "rss3")]
    Rss3, // = "rss3",

    /// https://docs.knn3.xyz/graphql/
    #[strum(serialize = "knn3")]
    #[serde(rename = "knn3")]
    #[graphql(name = "knn3")]
    Knn3, // = "knn3",

    #[strum(serialize = "cyberconnect")]
    #[serde(rename = "cyberconnect")]
    #[graphql(name = "cyberconnect")]
    CyberConnect,

    #[strum(serialize = "ethLeaderboard")]
    #[serde(rename = "ethLeaderboard")]
    #[graphql(name = "ethLeaderboard")]
    EthLeaderboard,

    #[strum(serialize = "the_graph")]
    #[serde(rename = "the_graph")]
    #[graphql(name = "the_graph")]
    TheGraph,

    /// Data directly fetched from blockchain's RPC server.
    #[strum(serialize = "rpc_server")]
    #[serde(rename = "rpc_server")]
    #[graphql(name = "rpc_server")]
    RPCServer,

    /// .bit
    #[strum(serialize = "dotbit")]
    #[serde(rename = "dotbit")]
    #[graphql(name = "dotbit")]
    Dotbit,

    /// UnstoppableDomains
    #[strum(serialize = "unstoppabledomains")]
    #[serde(rename = "unstoppabledomains")]
    #[graphql(name = "unstoppabledomains")]
    UnstoppableDomains,

    /// .lens
    /// https://docs.lens.xyz/docs/api-links
    #[strum(serialize = "lens")]
    #[serde(rename = "lens")]
    #[graphql(name = "lens")]
    Lens,

    #[strum(serialize = "farcaster")]
    #[serde(rename = "farcaster")]
    #[graphql(name = "farcaster")]
    Farcaster,

    #[strum(serialize = "space_id")]
    #[serde(rename = "space_id")]
    #[graphql(name = "space_id")]
    SpaceId,

    /// opensea
    /// https://opensea.io
    #[strum(serialize = "opensea")]
    #[serde(rename = "opensea")]
    #[graphql(name = "opensea")]
    OpenSea,

    // twitter_hexagon
    #[strum(serialize = "twitter_hexagon")]
    #[serde(rename = "twitter_hexagon")]
    #[graphql(name = "twitter_hexagon")]
    TwitterHexagon,

    /// Uniswap
    /// https://uniswap.org/
    #[strum(serialize = "uniswap")]
    #[serde(rename = "uniswap")]
    #[graphql(name = "uniswap")]
    Uniswap,

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    #[default]
    Unknown,
}

pub fn vec_string_to_vec_datasource(vec_string: Vec<String>) -> Result<Vec<DataSource>, Error> {
    let datasource_result: Result<Vec<DataSource>, _> = vec_string
        .into_iter()
        .map(|p_string| p_string.parse())
        .collect();
    Ok(datasource_result?)
}
