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
    /// Twitter <-> Ethereum
    /// (according to official README)
    /// This repo contains a list of verified mappings that link
    /// Ethereum addresses with social profiles (Twitter supported currently).
    #[strum(serialize = "sybil", serialize = "uniswap")]
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
    /// Twitter <-> Ethereum
    /// RSS3 open database of their social bindings.
    /// Twitter is guarenteed by RSS3's OAuth, wallet is guarenteed by RSS3's own signature standard.
    /// Partial crypto-verifiable.
    #[strum(serialize = "rss3")]
    #[serde(rename = "rss3")]
    #[graphql(name = "rss3")]
    Rss3, // = "rss3",

    /// https://docs.knn3.xyz/graphql/
    #[strum(serialize = "knn3")]
    #[serde(rename = "knn3")]
    #[graphql(name = "knn3")]
    Knn3, // = "knn3",

    /// CyberConnect
    /// https://cyberconnect.me
    /// Twitter <-> Etheruem
    /// Twitter binding is guarenteed by CC's OAuth.
    /// Wallet binding signature is based on CC's own standard, which is crypto-verifiable.
    #[strum(serialize = "cyberconnect", serialize = "cyber")]
    #[serde(rename = "cyberconnect")]
    #[graphql(name = "cyberconnect")]
    CyberConnect,

    /// https://ethleaderboard.xyz/
    /// Twitter <-> Ethereum
    /// Cannot be verified. Based on twitter `display_name` and followers.
    #[strum(serialize = "ethLeaderboard", serialize = "web ens data")]
    #[serde(rename = "ethLeaderboard")]
    #[graphql(name = "ethLeaderboard")]
    EthLeaderboard,

    /// Twitter <-> Ethereum
    /// ENS data fetched from twitter user's `screen_name`.
    /// (i.e., user changed their name as `seems-to-be-like-a.eth`)
    /// Pretty much unreliable.
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    #[graphql(name = "ens")]
    ENS,

    #[strum(serialize = "the_graph")]
    #[serde(rename = "the_graph")]
    #[graphql(name = "the_graph")]
    TheGraph,

    /// Data directly fetched from blockchain's RPC server, by calling contract's `public view` function.
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

    #[strum(serialize = "crossbell")]
    #[serde(rename = "crossbell")]
    #[graphql(name = "crossbell")]
    Crossbell,

    /// Clusters
    #[strum(serialize = "clusters")]
    #[serde(rename = "clusters")]
    #[graphql(name = "clusters")]
    Clusters,

    /// Solana
    #[strum(serialize = "solana")]
    #[serde(rename = "solana")]
    #[graphql(name = "solana")]
    Solana,

    /// Solana Name Service | Bonfida
    /// Bonfida is building top-tier infrastructure to enhance the efficiency of Solana with a community-centric approach.
    /// https://www.sns.id/
    #[strum(serialize = "sns")]
    #[serde(rename = "sns")]
    #[graphql(name = "sns")]
    SNS,

    /// Basenames
    /// Basenames are a core onchain building block that enable builders to establish their identity on Base by registering human-readable names for their wallet address(es).
    /// https://www.base.org/names
    #[strum(serialize = "basenames")]
    #[serde(rename = "basenames")]
    #[graphql(name = "basenames")]
    Basenames,

    /// opensea
    /// https://opensea.io
    /// Twitter <-> Ethereum
    /// Kinda not that trustable. In the old time, user can
    /// set their Twitter account in Opensea without any validation.
    /// Currently we cannot tell if a record has been validated by OpenSea.
    #[strum(serialize = "opensea")]
    #[serde(rename = "opensea")]
    #[graphql(name = "opensea")]
    OpenSea,

    /// twitter_hexagon
    /// Twitter <-> Ethereum
    /// NFT set by twitter user (hexagon PFP).
    /// We cannot get the original signature generated by user, so not verifiable.
    #[strum(serialize = "twitter_hexagon")]
    #[serde(rename = "twitter_hexagon")]
    #[graphql(name = "twitter_hexagon")]
    TwitterHexagon,

    /// Firefly
    /// https://firefly.land
    /// Twitter <-> Ethereum
    /// Firefly app has Twitter OAuth login info, and do binding with
    /// user's wallet by EIP-4361, which is crypto-guarenteed.
    #[strum(serialize = "firefly")]
    #[serde(rename = "firefly")]
    #[graphql(name = "firefly")]
    Firefly,

    /// Twitter <-> Ethereum
    /// Blocktracker's algorithm,
    /// by comparing Twitter user's profile pic (not hexagon PFP, but original pic) with existd NFT picture.
    /// Not verifiable, even has potential of mismatching.
    #[strum(serialize = "pfp")]
    #[serde(rename = "pfp")]
    #[graphql(name = "pfp")]
    PFP,

    /// Twitter <-> Ethereum
    /// Manually added by Firefly.land team.
    /// Cannot be verified by third party, only trust the team.
    #[strum(serialize = "manually_added")]
    #[serde(rename = "manually_added")]
    #[graphql(name = "manually_added")]
    ManuallyAdded,

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
