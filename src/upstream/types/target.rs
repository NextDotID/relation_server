use http::StatusCode;

use crate::{
    error::Error,
    graph::vertex::contract::{Chain, ContractCategory},
};

use super::platform::Platform;

/// List when processing identities.
pub type TargetProcessedList = Vec<Target>;

/// Target to fetch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// `Identity(platform, identity)`
    Identity(Platform, String),

    /// `NFT(chain, category, contract_address, NFT_ID)`
    NFT(Chain, ContractCategory, String, String),
}
impl Default for Target {
    fn default() -> Self {
        Target::Identity(Platform::default(), "".to_string())
    }
}
impl Target {
    /// Judge if this target is in supported platforms list given by upstream.
    pub fn in_platform_supported(&self, platforms: Vec<Platform>) -> bool {
        match self {
            Self::NFT(_, _, _, _) => false,
            Self::Identity(platform, _) => platforms.contains(platform),
        }
    }

    /// Judge if this target is in supported NFT category / chain list given by upstream.
    pub fn in_nft_supported(
        &self,
        nft_categories: Vec<ContractCategory>,
        nft_chains: Vec<Chain>,
    ) -> bool {
        match self {
            Self::Identity(_, _) => false,
            Self::NFT(chain, category, _, _) => {
                nft_categories.contains(category) && nft_chains.contains(chain)
            }
        }
    }

    pub fn platform(&self) -> Result<Platform, Error> {
        match self {
            Self::Identity(platform, _) => Ok(*platform),
            Self::NFT(_, _, _, _) => Err(Error::General(
                "Target: Get platform error: Not an Identity".into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        }
    }

    pub fn identity(&self) -> Result<String, Error> {
        match self {
            Self::Identity(_, identity) => Ok(identity.clone()),
            Self::NFT(_, _, _, _) => Err(Error::General(
                "Target: Get identity error: Not an Identity".into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        }
    }

    #[allow(dead_code)]
    pub fn nft_chain(&self) -> Result<Chain, Error> {
        match self {
            Self::Identity(_, _) => Err(Error::General(
                "Target: Get nft chain error: Not an NFT".into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
            Self::NFT(chain, _, _, _) => Ok(*chain),
        }
    }

    #[allow(dead_code)]
    pub fn nft_category(&self) -> Result<ContractCategory, Error> {
        match self {
            Self::Identity(_, _) => Err(Error::General(
                "Target: Get nft category error: Not an NFT".into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
            Self::NFT(_, category, _, _) => Ok(*category),
        }
    }

    #[allow(dead_code)]
    pub fn nft_id(&self) -> Result<String, Error> {
        match self {
            Self::Identity(_, _) => Err(Error::General(
                "Target: Get nft id error: Not an NFT".into(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
            Self::NFT(_, _, _, nft_id) => Ok(nft_id.clone()),
        }
    }
}
impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(platform, identity) => write!(f, "Identity/{}/{}", platform, identity),
            Self::NFT(chain, category, address, nft_id) => {
                write!(f, "NFT/{}/{}/{}/{}", chain, category, address, nft_id)
            }
        }
    }
}
