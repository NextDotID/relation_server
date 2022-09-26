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

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    #[default]
    Unknown,
}
