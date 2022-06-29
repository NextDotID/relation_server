mod aggregation;
mod keybase;
mod knn3;
mod proof_client;
mod rss3;
mod sybil_list;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::{
    error::Error,
    graph::vertex::IdentityRecord,
    graph::{
        edge::{Own, ProofRecord},
        vertex::NFTRecord,
    },
};

/// All identity platform.
#[derive(Serialize, Deserialize, Debug, EnumString, Clone, Display, PartialEq)]
pub enum Platform {
    /// Twitter
    #[strum(serialize = "twitter")]
    #[serde(rename = "twitter")]
    Twitter,

    /// Ethereum wallet `0x[a-f0-9]{40}`
    #[strum(serialize = "ethereum", serialize = "eth")]
    #[serde(rename = "ethereum")]
    Ethereum,

    /// NextID
    #[strum(serialize = "nextid")]
    #[serde(rename = "nextid")]
    NextID,

    /// Keybase
    #[strum(serialize = "keybase")]
    #[serde(rename = "keybase")]
    Keybase,

    /// Github
    #[strum(serialize = "github")]
    #[serde(rename = "github")]
    Github,

    /// ENS
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    Ens,

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    Unknown,
}

/// All data respource platform.
#[derive(Serialize, Deserialize, Debug, Clone, Display, EnumString, PartialEq)]
pub enum DataSource {
    /// https://github.com/Uniswap/sybil-list/blob/master/verified.json
    #[strum(serialize = "sybil")]
    #[serde(rename = "sybil")]
    SybilList,

    /// https://keybase.io/docs/api/1.0/call/user/lookup
    #[strum(serialize = "keybase")]
    #[serde(rename = "keybase")]
    Keybase,

    /// https://docs.next.id/docs/proof-service/api
    #[strum(serialize = "nextid")]
    #[serde(rename = "nextid")]
    NextID, // = "nextID",

    /// https://rss3.io/network/api.html
    #[strum(serialize = "rss3")]
    #[serde(rename = "rss3")]
    Rss3, // = "rss3",

    #[strum(serialize = "cyberconnect")]
    #[serde(rename = "cyberconnect")]
    CyberConnect,

    #[strum(serialize = "ethLeaderboard")]
    #[serde(rename = "ethLeaderboard")]
    EthLeaderboard,

    /// Unknow
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for DataSource {
    fn default() -> Self {
        DataSource::NextID
    }
}

/// All asymmetric cryptography algorithm supported by RelationService.
#[derive(Serialize, Deserialize)]
pub enum Algorithm {
    EllipticCurve,
}

/// All elliptic curve supported by RelationService.
#[derive(Serialize, Deserialize)]
pub enum Curve {
    Secp256K1,
}

/// EdgeType: PubkeySerialize
#[derive(Debug)]
pub struct TempPubkeySerialize {
    pub uuid: uuid::Uuid,
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Connection {
    pub from: IdentityRecord,
    pub to: IdentityRecord,
    pub proof: ProofRecord,
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Owned {
    pub from: IdentityRecord,
    pub to: NFTRecord,
    pub proof: Own,
}

/// Fetcher defines how to fetch data from upstream.
#[async_trait]
pub trait Fetcher {
    /// Fetch data from given source.
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error>;

    /// return support platform vec
    fn ability() -> Vec<(Vec<Platform>, Vec<Platform>)>;
}
