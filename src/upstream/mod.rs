mod proof_client;
mod sybil_list;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::error::Error;

/// All identity platform.
#[derive(Serialize, Deserialize, Debug, EnumString, Clone, Display)]
pub enum Platform {
    /// Twitter
    #[strum(serialize = "twitter")]
    #[serde(rename = "twitter")]
    Twitter,
    /// Ethereum wallet `0x[a-f0-9]{40}`
    #[strum(serialize = "ethereum")]
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
}

/// All data respource platform.
#[derive(Serialize, Deserialize, Debug, Clone, Display, EnumString)]
pub enum DataSource {
    /// https://github.com/Uniswap/sybil-list/blob/master/verified.json
    #[strum(serialize = "sybil_list")]
    #[serde(rename = "sybil_list")]
    SybilList,
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

/// TODO: use DB-defined struct instead.
/// VertexType: Identity
#[derive(Debug)]
pub struct TempIdentity {
    pub uuid: uuid::Uuid,
    pub platform: Platform,
    pub identity: String,
    pub created_at: Option<NaiveDateTime>,
    pub display_name: Option<String>,
}

/// TODO: use DB-defined struct instead.
/// VertexType: CryptoIdentity
pub struct TempCryptoIdentity {
    pub uuid: uuid::Uuid,
    /// 0xHEXSTRING, no compression.
    pub public_key: String,
    pub algorithm: Algorithm,
    pub curve: Curve,
}

/// EdgeType: Proof
#[derive(Debug)]
pub struct TempProof {
    pub uuid: uuid::Uuid,
    pub method: DataSource,
    /// 通常为 URL，同一个 fetcher 可以对接不同上游的场景
    pub upstream: Option<String>,
    pub record_id: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub last_verified_at: NaiveDateTime,
}

/// EdgeType: PubkeySerialize
#[derive(Debug)]
pub struct TempPubkeySerialize {
    pub uuid: uuid::Uuid,
}

/// Info of a complete binding.
#[derive(Debug)]
pub struct Connection {
    pub from: TempIdentity,
    pub to: TempIdentity,
    pub proof: TempProof,
}

/// Fetcher defines how to fetch data from upstream.
#[async_trait]
pub trait Fetcher {
    /// Fetch data from given source.
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error>;
}
