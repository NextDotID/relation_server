use crate::upstream::Chain;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    Debug,
    EnumIter,
    PartialEq,
    Eq,
    async_graphql::Enum,
    Hash,
)]
pub enum ContractCategory {
    #[strum(serialize = "ENS")]
    #[serde(rename = "ENS")]
    #[graphql(name = "ENS")]
    ENS,

    #[strum(serialize = "ERC721", serialize = "ERC-721")]
    #[serde(rename = "ERC721")]
    #[graphql(name = "ERC721")]
    ERC721,

    #[strum(serialize = "ERC1155", serialize = "ERC-1155")]
    #[serde(rename = "ERC1155")]
    #[graphql(name = "ERC1155")]
    ERC1155,

    #[strum(serialize = "POAP")]
    #[serde(rename = "POAP")]
    #[graphql(name = "POAP")]
    POAP,

    #[default]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    #[strum(serialize = "unknown")]
    Unknown,
}

impl ContractCategory {
    pub fn default_contract_address(&self) -> Option<String> {
        use ContractCategory::*;
        match self {
            // TODO: ENS has a complicated contract structure, which cannot determine the "main" contract easily.
            ENS => Some("0x57f1887a8BF19b14fC0dF6Fd9B2acc9Af147eA85".to_lowercase()),
            _ => None,
        }
    }

    pub fn default_chain(&self) -> Option<Chain> {
        use ContractCategory::*;
        match self {
            ENS => Some(Chain::Ethereum),
            ERC721 => Some(Chain::Ethereum),
            ERC1155 => Some(Chain::Ethereum),
            POAP => Some(Chain::Ethereum),
            _ => None,
        }
    }
}
