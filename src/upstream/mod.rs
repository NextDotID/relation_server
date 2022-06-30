mod aggregation;
mod keybase;
mod knn3;
mod proof_client;
mod rss3;
mod sybil_list;

use crate::upstream::aggregation::Aggregation;
use crate::upstream::keybase::Keybase;
use crate::upstream::proof_client::ProofClient;
use crate::upstream::sybil_list::SybilList;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
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

    /// https://docs.knn3.xyz/graphql/
    #[strum(serialize = "knn3")]
    #[serde(rename = "knn3")]
    Knn3, // = "rss3",

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

/// Fetcher defines how to fetch data from upstream.
#[async_trait]
pub trait Fetcher {
    /// Fetch data from given source.
    async fn fetch(&self) -> Result<(), Error>;

    /// return support platform vec
    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)>;
}

#[derive(EnumIter, Debug, PartialEq)]
enum Upstream {
    Keybase,
    NextID,
    SybilList,
    Aggregation,
}

struct upstreamFactory;

impl upstreamFactory {
    fn new_fetcher(u: &Upstream, platform: String, identity: String) -> Box<dyn Fetcher> {
        match u {
            Upstream::Keybase => Box::new(Keybase {
                platform: platform.clone(),
                identity: identity.clone(),
            }),
            Upstream::NextID => Box::new(ProofClient {
                platform: platform.clone(),
                identity: identity.clone(),
            }),
            Upstream::SybilList => Box::new(SybilList {}),
            Upstream::Aggregation => Box::new(Aggregation {
                platform: platform.clone(),
                identity: identity.clone(),
            }),
        }
    }
}

///fetch data from all supported upstream
async fn fetch_all(platform: String, identity: String) -> Result<(), Error> {
    let mut data_fetch: Box<dyn Fetcher>;
    let mut ability: Vec<(Vec<Platform>, Vec<Platform>)>;
    //let mut result = Vec::new();

    for source in Upstream::iter() {
        data_fetch = upstreamFactory::new_fetcher(&source, platform.clone(), identity.clone());
        ability = data_fetch.ability();
        for (support_platforms, _) in ability.into_iter() {
            if support_platforms.iter().any(|p| p.to_string() == platform) {
                let res = data_fetch.fetch().await;
                if res.is_err() {
                    continue;
                }
            }
        }
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::fetch_all;

    #[tokio::test]
    async fn test_fetcher_result() -> Result<(), Error> {
        let result = fetch_all("github".to_string(), "fengshanshan".to_string()).await?;
        assert_eq!(result, ());
        Ok(())
    }
}
