use crate::upstream::DomainNameSystem;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// All identity platform.
/// TODO: move this definition into `graph/vertex/identity`, since it is not specific to upstream.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    EnumString,
    Clone,
    Copy,
    Display,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Hash,
    async_graphql::Enum,
)]
pub enum Platform {
    /// Twitter
    #[strum(serialize = "twitter")]
    #[serde(rename = "twitter")]
    #[graphql(name = "twitter")]
    Twitter,

    /// Bitcoin
    #[strum(serialize = "bitcoin")]
    #[serde(rename = "bitcoin")]
    #[graphql(name = "bitcoin")]
    Bitcoin,

    /// Ethereum wallet `0x[a-f0-9]{40}`
    #[strum(serialize = "ethereum", serialize = "eth")]
    #[serde(rename = "ethereum")]
    #[graphql(name = "ethereum")]
    Ethereum,

    /// Solana
    #[strum(serialize = "solana")]
    #[serde(rename = "solana")]
    #[graphql(name = "solana")]
    Solana,

    /// NextID
    #[strum(serialize = "nextid")]
    #[serde(rename = "nextid")]
    #[graphql(name = "nextid")]
    NextID,

    /// Keybase
    #[strum(serialize = "keybase")]
    #[serde(rename = "keybase")]
    #[graphql(name = "keybase")]
    Keybase,

    /// Github
    #[strum(serialize = "github")]
    #[serde(rename = "github")]
    #[graphql(name = "github")]
    Github,

    /// Reddit
    #[strum(serialize = "reddit")]
    #[serde(rename = "reddit")]
    #[graphql(name = "reddit")]
    Reddit,

    /// Facebook
    #[strum(serialize = "facebook")]
    #[serde(rename = "facebook")]
    #[graphql(name = "facebook")]
    Facebook,

    /// Instagram
    #[strum(serialize = "instagram")]
    #[serde(rename = "instagram")]
    #[graphql(name = "instagram")]
    Instagram,

    /// Mastodon maintained by Sujitech
    #[strum(serialize = "mstdnjp")]
    #[serde(rename = "mstdnjp")]
    #[graphql(name = "mstdnjp")]
    MstdnJP,

    /// Lobsters is a computing-focused community centered around link aggregation and discussion
    #[strum(serialize = "lobsters")]
    #[serde(rename = "lobsters")]
    #[graphql(name = "lobsters")]
    Lobsters,

    /// The Hacker News is the most trusted and popular cybersecurity publication for information security professionals seeking breaking news.
    #[strum(serialize = "hackernews")]
    #[serde(rename = "hackernews")]
    #[graphql(name = "hackernews")]
    HackerNews,

    /// ENS: ENS domains provide a way for users to map human readable names to blockchain and non-blockchain resources.
    /// https://ens.domains/
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    #[graphql(name = "ens")]
    ENS,

    /// Solana Name Service: Create a human-readable identity by replacing decentralized addresses with a domain name.
    /// https://www.sns.id
    #[strum(serialize = "sns")]
    #[serde(rename = "sns")]
    #[graphql(name = "sns")]
    SNS,

    /// Lens: Lens is an open social network where users own their content and connections.
    /// https://www.lens.xyz/
    #[strum(serialize = "Lens", serialize = "lens")]
    #[serde(rename = "lens")]
    #[graphql(name = "lens")]
    Lens,

    /// .bit: A protocols for proof of humanity and achievement network,
    /// connecting every human. Own your ID and achievement through our blockchain-powered protocol network
    /// https://d.id/
    #[strum(serialize = "dotbit")]
    #[serde(rename = "dotbit")]
    #[graphql(name = "dotbit")]
    Dotbit,

    /// DNS
    #[strum(serialize = "dns")]
    #[serde(rename = "dns")]
    #[graphql(name = "dns")]
    DNS,

    /// Minds: Interoperable with web2 and web3 protocols like ActivityPub, RSS, DNS, Bitcoin, Ethereum, Stripe and more.
    /// https://www.minds.com/
    #[strum(serialize = "minds")]
    #[serde(rename = "minds")]
    #[graphql(name = "minds")]
    Minds,

    /// UnstoppableDomains: One Stop Shop for Onchain Domains
    /// https://unstoppabledomains.com/
    #[strum(serialize = "unstoppabledomains")]
    #[serde(rename = "unstoppabledomains")]
    #[graphql(name = "unstoppabledomains")]
    UnstoppableDomains,

    /// Farcaster: Farcaster is a fully decentralized social network.
    /// https://www.farcaster.xyz/
    #[strum(serialize = "farcaster")]
    #[serde(rename = "farcaster")]
    #[graphql(name = "farcaster")]
    Farcaster,

    /// SpaceId: A Web3 Identity Protocol with Multi-chain Name Service.
    /// equip communities with powerful tools to launch their desired Top-Level-Domain
    /// https://space.id/
    #[strum(serialize = "space_id")]
    #[serde(rename = "space_id")]
    #[graphql(name = "space_id")]
    SpaceId,

    /// Genome: .GNO domains for your web3 identity.
    /// community-owned network that prioritizes credible neutrality and resiliency.
    /// https://genomedomains.com/
    #[strum(serialize = "genome")]
    #[serde(rename = "genome")]
    #[graphql(name = "genome")]
    Genome,

    /// Crossbell: Crossbell is a social ownership platform to build cutting-edge social dApps.
    /// https://crossbell.io/
    #[strum(serialize = "crossbell")]
    #[serde(rename = "crossbell")]
    #[graphql(name = "crossbell")]
    Crossbell,

    /// CKB: Common Knowledge Base
    /// https://www.nervos.org/
    #[strum(serialize = "ckb")]
    #[serde(rename = "ckb")]
    #[graphql(name = "ckb")]
    CKB,

    /// TRON Network: An ambitious project dedicated to building the infrastructure
    /// for a truly decentralized Internet.
    /// https://tron.network/
    #[strum(serialize = "tron")]
    #[serde(rename = "tron")]
    #[graphql(name = "tron")]
    Tron,

    /// TON Network: A decentralized and open internet,
    /// created by the community using a technology designed by Telegram.
    /// https://ton.org/
    #[strum(serialize = "ton")]
    #[serde(rename = "ton")]
    #[graphql(name = "ton")]
    Ton,

    /// Doge: https://dogechain.dog/
    #[strum(serialize = "doge")]
    #[serde(rename = "doge")]
    #[graphql(name = "doge")]
    Doge,

    /// BNB Smart Chain (BSC)
    /// https://docs.bnbchain.org/bnb-smart-chain/overview/
    #[strum(serialize = "bsc")]
    #[serde(rename = "bsc")]
    #[graphql(name = "bsc")]
    BNBSmartChain,

    /// Polygon
    /// https://www.polygon.com/
    #[serde(rename = "polygon")]
    #[strum(serialize = "polygon")]
    #[graphql(name = "polygon")]
    Polygon,

    /// Clusters: Clusters is the leading universal name service. Every blockchain, all your wallets, one name.
    /// The dominant LayerZero name service.
    /// https://docs.clusters.xyz/
    #[serde(rename = "clusters")]
    #[strum(serialize = "clusters")]
    #[graphql(name = "clusters")]
    Clusters,

    /// Aptos: Aptos is an independent Layer 1 blockchain platform focused on safety and
    /// scalability driving growth within a decentralized network and developer ecosystem.
    /// https://aptosfoundation.org/
    #[serde(rename = "aptos")]
    #[strum(serialize = "aptos")]
    #[graphql(name = "aptos")]
    Aptos,

    /// Near: NEAR is the chain abstraction stack, empowering builders to create apps
    /// that scale to billions of users and across all blockchains.
    /// https://near.org/
    #[serde(rename = "near")]
    #[strum(serialize = "near")]
    #[graphql(name = "near")]
    Near,

    /// Stacks: The Leading Bitcoin L2 for Smart Contracts, Apps, DeFi.
    /// https://www.stacks.co/
    #[serde(rename = "stacks")]
    #[strum(serialize = "stacks")]
    #[graphql(name = "stacks")]
    Stacks,

    /// Xrpc: Xrp Classic's purpose is to develop eco-friendly solutions
    /// that will make the cryptocurrency space safer and easier to understand for everyone.
    /// https://www.xrpclassic.com/
    #[serde(rename = "xrpc")]
    #[strum(serialize = "xrpc")]
    #[graphql(name = "xrpc")]
    Xrpc,

    /// Cosmos: Cosmos is an ever-expanding ecosystem of interoperable and sovereign blockchain appsand services,
    /// built for a decentralized future.
    /// https://cosmos.network/
    #[serde(rename = "cosmos")]
    #[strum(serialize = "cosmos")]
    #[graphql(name = "cosmos")]
    Cosmos,

    /// Unknown
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    #[default]
    Unknown,
}

impl From<Platform> for DomainNameSystem {
    fn from(platform: Platform) -> Self {
        match platform {
            Platform::Dotbit => DomainNameSystem::DotBit,
            Platform::UnstoppableDomains => DomainNameSystem::UnstoppableDomains,
            Platform::Lens => DomainNameSystem::Lens,
            Platform::SpaceId => DomainNameSystem::SpaceId,
            Platform::Crossbell => DomainNameSystem::SpaceId,
            Platform::ENS => DomainNameSystem::ENS,
            Platform::SNS => DomainNameSystem::SNS,
            Platform::Genome => DomainNameSystem::Genome,
            Platform::Clusters => DomainNameSystem::Clusters,
            _ => DomainNameSystem::Unknown,
        }
    }
}
