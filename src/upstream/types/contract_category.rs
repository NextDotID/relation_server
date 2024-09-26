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
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    #[graphql(name = "ens")]
    ENS,

    #[strum(serialize = "erc721")]
    #[serde(rename = "erc721")]
    #[graphql(name = "erc721")]
    ERC721,

    #[strum(serialize = "erc1155")]
    #[serde(rename = "erc1155")]
    #[graphql(name = "erc1155")]
    ERC1155,

    #[strum(serialize = "poap")]
    #[serde(rename = "poap")]
    #[graphql(name = "poap")]
    POAP,

    #[strum(serialize = "sns")]
    #[serde(rename = "sns")]
    #[graphql(name = "sns")]
    SNS,

    #[strum(serialize = "gns")]
    #[serde(rename = "gns")]
    #[graphql(name = "gns")]
    GNS,

    #[strum(serialize = "basenames")]
    #[serde(rename = "basenames")]
    #[graphql(name = "basenames")]
    Basenames,

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
            SNS => Some("4gj2A7SSgWUGfHTm2iG4NeH3kpySmGd54bj78TM4d7Fg".to_string()), // Solana Name Service
            GNS => Some("0x5dc881dda4e4a8d312be3544ad13118d1a04cb17".to_string()), // Gnosis Name Service
            Basenames => Some("0x4ccb0bb02fcaba27e82a56646e81d8c5bc4119a5".to_string()), // Basenames
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
            SNS => Some(Chain::Solana),
            GNS => Some(Chain::Gnosis),
            Basenames => Some(Chain::Base),
            _ => None,
        }
    }
}
