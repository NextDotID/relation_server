mod sybil_list;
mod proof_client;
mod keybase;


use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use strum_macros::EnumString;

use crate::error::Error;


pub enum VertexType {
    Identity,
    CryptoIdentity,
}

pub enum EdgeType {
    Proof,
    PubkeySerialize,
}
/// All identity platform.
#[derive(Serialize, Deserialize, Debug, EnumString)]
pub enum Platform {
    /// Twitter
    #[strum(serialize = "twitter")]
    Twitter,
    /// Ethereum wallet. (0x[a-f0-9]{40})
    #[strum(serialize = "ethereum")]
    Ethereum,
    /// NextID
    #[strum(serialize = "nextid")]
    NextID,
    /// Keybase
    #[strum(serialize = "keybase")]
    Keybase,
    /// Github
    #[strum(serialize = "github")]
    Github,
}

/// All data respource platform.
#[derive(Serialize, Deserialize, Debug)]
pub enum DataSource {
    /// https://github.com/Uniswap/sybil-list/blob/master/verified.json
    SybilList, // = "sybil_list",

    /// https://keybase.io/docs/api/1.0/call/user/lookup
    Keybase, // = "keybase",

    /// https://docs.next.id/docs/proof-service/api
    NextID, // = "nextID",
}

#[derive(Serialize, Deserialize)]
pub enum Algorithm {
    EllipticCurve,
}

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


