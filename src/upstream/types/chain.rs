use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// List of chains supported by RelationService.
#[derive(
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Debug,
    Display,
    PartialEq,
    Eq,
    async_graphql::Enum,
    EnumString,
    EnumIter,
    Hash,
)]
pub enum Chain {
    /// The Blockchain.
    #[serde(rename = "ethereum")]
    #[strum(serialize = "ethereum")]
    #[graphql(name = "ethereum")]
    Ethereum,

    /// Deprecated since `The Merge`.
    #[serde(rename = "rinkeby")]
    #[strum(serialize = "rinkeby")]
    #[graphql(name = "rinkeby")]
    Rinkeby,

    /// Deprecated since `The Merge`.
    #[serde(rename = "ropsten")]
    #[strum(serialize = "ropsten")]
    #[graphql(name = "ropsten")]
    Ropsten,

    /// Deprecated since `The Merge`.
    #[serde(rename = "kovan")]
    #[strum(serialize = "kovan")]
    #[graphql(name = "kovan")]
    Kovan,

    /// A cross-client proof-of-authority testing network for Ethereum.
    /// https://goerli.net
    #[serde(rename = "goerli")]
    #[strum(serialize = "goerli")]
    #[graphql(name = "goerli")]
    Goerli,

    /// Sepolia is expected to undergo `The Merge` to proof-of-stake in summer 2022.
    /// https://sepolia.dev
    #[serde(rename = "sepolia")]
    #[strum(serialize = "sepolia")]
    #[graphql(name = "sepolia")]
    Sepolia,

    /// BNB Smart Chain (BSC) (Previously Binance Smart Chain) - EVM compatible, consensus layers, and with hubs to multi-chains.
    /// https://www.binance.com/en/support/announcement/854415cf3d214371a7b60cf01ead0918
    #[serde(rename = "bsc")]
    #[strum(serialize = "bsc", serialize = "binance_smart_chain")]
    #[graphql(name = "bsc")]
    BNBSmartChain,

    /// Polygon is a decentralised Ethereum scaling platform that enables developers to build scalable user-friendly dApps with low transaction fees without ever sacrificing on security.
    /// https://polygon.technology
    #[serde(rename = "polygon")]
    #[strum(serialize = "polygon")]
    #[graphql(name = "polygon")]
    Polygon,

    /// Polygon Testnet
    /// https://mumbai.polygonscan.com
    #[serde(rename = "mumbai")]
    #[strum(serialize = "mumbai")]
    #[graphql(name = "mumbai")]
    Mumbai,

    /// Solana is a decentralized blockchain built to enable scalable, user-friendly apps for the world.
    /// https://solana.com
    #[serde(rename = "solana")]
    #[strum(serialize = "solana")]
    #[graphql(name = "solana")]
    Solana,

    /// Conflux is a new secure and reliable public blockchain with very high performance and scalability.
    /// https://developer.confluxnetwork.org
    #[serde(rename = "conflux")]
    #[strum(serialize = "conflux")]
    #[graphql(name = "conflux")]
    Conflux,

    /// Conflux has a virtual machine that is similar to the EVM.
    /// https://evm.confluxscan.io
    /// https://developer.confluxnetwork.org/conflux-doc/docs/EVM-Space/intro_of_evm_space
    #[serde(rename = "conflux_espace")]
    #[strum(serialize = "conflux_espace")]
    #[graphql(name = "conflux_espace")]
    ConfluxESpace,

    #[serde(rename = "ethereum_classic")]
    #[strum(serialize = "ethereum_classic")]
    #[graphql(name = "ethereum_classic")]
    EthereumClassic,

    /// https://zksync.io
    #[serde(rename = "zksync")]
    #[strum(serialize = "zksync")]
    #[graphql(name = "zksync")]
    ZKSync,

    #[serde(rename = "xdai")]
    #[strum(serialize = "xdai")]
    #[graphql(name = "xdai")]
    Xdai,
    /// Gnosis Chain provides stability, scalability and an extendable beacon chain framework.
    /// Established in 2018 as the xDai Chain, the updated Gnosis Chain gives devs the tools and resources they need to create enhanced user experiences and optimized applications.
    /// https://developers.gnosischain.com
    #[serde(rename = "gnosis")]
    #[strum(serialize = "gnosis")]
    #[graphql(name = "gnosis")]
    Gnosis,

    /// Arweave enables you to store documents and applications forever.
    /// https://www.arweave.org
    #[serde(rename = "arweave")]
    #[strum(serialize = "arweave")]
    #[graphql(name = "arweave")]
    Arweave,

    /// Arbitrum One
    /// http://arbiscan.io
    #[serde(rename = "arbitrum")]
    #[strum(serialize = "arbitrum")]
    #[graphql(name = "arbitrum")]
    Arbitrum,

    /// Optimism is a low-cost and lightning-fast Ethereum L2 blockchain.
    /// https://www.optimism.io
    #[serde(rename = "optimism")]
    #[strum(serialize = "optimism")]
    #[graphql(name = "optimism")]
    Optimism,

    #[serde(rename = "crossbell")]
    #[strum(serialize = "crossbell")]
    #[graphql(name = "crossbell")]
    Crossbell,

    /// Avalanche is an open, programmable smart contracts platform for decentralized applications.
    /// https://www.avax.com/
    #[serde(rename = "avalanche")]
    #[strum(serialize = "avalanche")]
    #[graphql(name = "avalanche")]
    Avalanche,

    /// Fantom is a highly scalable blockchain platform for DeFi, crypto dApps, and enterprise applications.
    /// https://fantom.foundation/
    #[serde(rename = "fantom")]
    #[strum(serialize = "fantom")]
    #[graphql(name = "fantom")]
    Fantom,

    /// Celo is the carbon-negative, mobile-first, EVM-compatible blockchain ecosystem leading a thriving new digital economy for all.
    /// https://celo.org/
    #[serde(rename = "celo")]
    #[strum(serialize = "celo")]
    #[graphql(name = "celo")]
    Celo,

    /// CKB
    /// https://www.nervos.org/ Common Knowledge Base
    #[strum(serialize = "ckb")]
    #[serde(rename = "ckb")]
    #[graphql(name = "ckb")]
    CKB,

    /// Base is an easy way for decentralized apps to leverage Coinbase's products and distribution.
    /// https://base.org/
    /// https://basescan.org/
    #[serde(rename = "base")]
    #[strum(serialize = "base")]
    #[graphql(name = "base")]
    Base,

    #[default]
    #[serde(rename = "unknown")]
    #[strum(serialize = "unknown")]
    #[graphql(name = "unknown")]
    Unknown,
}

/// Internal chain implementation / framework.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ChainType {
    /// EVM (with its chain ID)
    EVM(u128),
    Solana,
    /// Seems like an Layer2 of Ethereum?
    ZKSync,
    /// Arweave
    Arweave,
    /// Basiclly an EVM, but with different address serializer, transaction packaging and genesis contracts.
    Conflux,
}

impl Default for ChainType {
    fn default() -> Self {
        Chain::default().chain_type()
    }
}

impl Chain {
    /// Returns chain implementation / framework.
    pub fn chain_type(&self) -> ChainType {
        use Chain::*;

        match self {
            Ethereum => ChainType::EVM(1),
            Rinkeby => ChainType::EVM(4),
            Ropsten => ChainType::EVM(3),
            Kovan => ChainType::EVM(42),
            Goerli => ChainType::EVM(5),
            Sepolia => ChainType::EVM(11155111),
            BNBSmartChain => ChainType::EVM(56),
            Polygon => ChainType::EVM(137),
            Mumbai => ChainType::EVM(80001),
            Solana => ChainType::Solana,
            Conflux => ChainType::Conflux,
            ConfluxESpace => ChainType::EVM(71),
            EthereumClassic => ChainType::EVM(61),
            ZKSync => ChainType::ZKSync,
            Xdai => ChainType::EVM(100),
            Gnosis => ChainType::EVM(100),
            Arweave => ChainType::Arweave,
            Arbitrum => ChainType::EVM(42161),
            Optimism => ChainType::EVM(10),
            Crossbell => ChainType::EVM(3737),
            Avalanche => ChainType::EVM(43114),
            Fantom => ChainType::EVM(250),
            Celo => ChainType::EVM(42220),
            CKB => ChainType::EVM(71402),
            Base => ChainType::EVM(8453),
            Unknown => todo!(),
        }
    }
}
