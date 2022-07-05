pub mod aggregation;
pub mod keybase;
pub mod proof_client;
pub mod rss3;
pub mod sybil_list;

use std::sync::Arc;

use crate::error::Error;
use crate::upstream::proof_client::ProofClient;
use crate::upstream::sybil_list::SybilList;
use crate::upstream::{aggregation::Aggregation, keybase::Keybase};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::{Display, EnumString};

/// List when processing identities.
type IdentityProcessList = Vec<(Platform, String)>;
//type IdentityObject = (Platform, String);

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

    #[strum(serialize = "cyberconnect")]
    #[serde(rename = "cyberconnect")]
    CyberConnect,

    #[strum(serialize = "ethLeaderboard")]
    #[serde(rename = "ethLeaderboard")]
    EthLeaderboard,

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for DataSource {
    fn default() -> Self {
        Self::Unknown
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

/// Fetcher defines how to fetch data from upstream.
#[async_trait]
pub trait Fetcher {
    /// Fetch data from given source.
    async fn fetch(&self) -> Result<IdentityProcessList, Error>;

    /// Ability for this upstream.
    /// `Vec<(AcceptedPlatformsAsInput, ResultOfPlatforms)>`
    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)>;
}

#[derive(EnumIter, Debug, PartialEq)]
enum Upstream {
    Keybase,
    NextID,
    SybilList,
    Aggregation,
}

struct UpstreamFactory;

impl UpstreamFactory {
    fn new_fetcher(
        u: &Upstream,
        platform: &String,
        identity: &String,
    ) -> Box<dyn Fetcher + Sync + Send> {
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

/// Find all available (platform, identity) in all `Upstream`s.
pub async fn fetch_all(platform: &Platform, identity: &String) -> Result<(), Error> {
    let mut up_next: IdentityProcessList = vec![(platform.clone(), identity.clone())];
    let mut processed: IdentityProcessList = vec![];
    while up_next.len() > 0 {
        let (next_platform, next_identity) = up_next.pop().unwrap();

        let fetched = fetch_one(&next_platform, &next_identity).await?;
        processed.push((next_platform, next_identity));
        fetched.clone().into_iter().for_each(|f| {
            if processed.iter().all(|p| p.0 != f.0 || p.1 != f.1) {
                up_next.push((f.0, f.1));
            }
        });
    }

    Ok(())
}

/// Find one (platform, identity) pair in all upstreams.
/// Returns identities just fetched for next iter..
pub async fn fetch_one(
    platform: &Platform,
    identity: &String,
) -> Result<IdentityProcessList, Error> {
    let mut res: IdentityProcessList = Vec::new();
    for source in Upstream::iter() {
        let fetcher = UpstreamFactory::new_fetcher(&source, &platform.to_string(), identity);
        let ability = fetcher.ability();
        for (platforms, _) in ability.into_iter() {
            if platforms.iter().any(|p| p == platform) {
                let resp = match fetcher.fetch().await {
                    Ok(resp) => resp,
                    Err(..) => continue,
                };
                res.extend(resp);
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::{fetch_all, Platform};

    #[tokio::test]
    async fn test_fetcher_result() -> Result<(), Error> {
        let result = fetch_all(&Platform::Github, &"fengshanshan".into()).await?;
        assert_eq!(result, ());

        Ok(())
    }
}
