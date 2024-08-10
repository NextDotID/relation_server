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

    /// Zeta: ZetaChain with its Universal EVM, ZetaChain is an L1 blockchain for chain abstraction.
    /// https://www.zetachain.com/
    #[strum(serialize = "zeta")]
    #[serde(rename = "zeta")]
    #[graphql(name = "zeta")]
    Zeta,

    /// Mode: Mode is developing the framework for AI agents to optimise onchain trading and yield generation strategies for users.
    /// https://www.mode.network/
    #[strum(serialize = "mode")]
    #[serde(rename = "mode")]
    #[graphql(name = "mode")]
    Mode,

    /// Arbitrum is the leading Layer 2 technology that empowers you to explore and build in the largest Layer 1 ecosystem, Ethereum.
    /// https://arbitrum.io/
    #[strum(serialize = "arbitrum")]
    #[serde(rename = "arbitrum")]
    #[graphql(name = "arbitrum")]
    Arbitrum,

    /// Scroll seamlessly extends Ethereum's capabilities through zero knowledge tech and EVM compatibility.
    /// The L2 network built by Ethereum devs for Ethereum devs.
    /// https://scroll.io/
    #[strum(serialize = "scroll")]
    #[serde(rename = "scroll")]
    #[graphql(name = "scroll")]
    Scroll,

    /// Taiko is a fully permissionless, Ethereum-equivalent based rollup. Inspired, secured, and sequenced by Ethereum.
    /// https://taiko.xyz/
    #[strum(serialize = "taiko")]
    #[serde(rename = "taiko")]
    #[graphql(name = "taiko")]
    Taiko,

    /// Mint: Focus on constructing Mint blockchain network and core components,
    /// including open source code for blockchain, NIP functions, MRC library, cross chain bridge, sorter, and other core functions.
    /// https://www.mintchain.io/
    #[strum(serialize = "mint")]
    #[serde(rename = "mint")]
    #[graphql(name = "mint")]
    Mint,

    /// ZKFair Mainnet
    /// ZKFair is the first ZK-Rollup on ethereum based on Polygon CDK and Celestia DA.
    /// https://zkfair.io/
    #[strum(serialize = "zkfair")]
    #[serde(rename = "zkfair")]
    #[graphql(name = "zkfair")]
    Zkfair,

    /// Manta Pacific Mainnet
    /// The first EVM-native modular execution layer for wide ZK applications adoption,
    /// with Manta’s universal circuit and zk interface.
    /// https://pacific.manta.network/
    #[strum(serialize = "manta")]
    #[serde(rename = "manta")]
    #[graphql(name = "manta")]
    Manta,

    /// LightLink Mainnet
    /// LightLink is an Ethereum Layer 2 blockchain that lets dApps and enterprises offer users instant, gasless transactions.
    /// https://lightlink.io/
    #[strum(serialize = "lightlink")]
    #[serde(rename = "lightlink")]
    #[graphql(name = "lightlink")]
    Lightlink,

    /// Merlin Chain supports popular Bitcoin protocols such as BRC20, BRC420, Bitmap, Atomicals, Pipe, Stamp, and more,
    /// enabling a more extensive user base to interact on Bitcoin Layer2.
    /// https://merlinchain.io/
    #[strum(serialize = "merlin")]
    #[serde(rename = "merlin")]
    #[graphql(name = "merlin")]
    Merlin,

    /// AlienX: AlienX is the blockchain infrastructure built for the large-scale adoption of AI, NFT, and Gaming.
    /// https://alienxchain.io
    #[strum(serialize = "alienx")]
    #[serde(rename = "alienx")]
    #[graphql(name = "alienx")]
    AlienX,

    /// Edgeless: Edgeless is the first ever crypto ecosystem without application layer fees.
    /// Edgeless is built as an L2 powered by Arbitrum Nitro and is the best place for builders and users to build and interact with decentralized applications.
    /// https://www.edgeless.network/
    #[strum(serialize = "edgeless")]
    #[serde(rename = "edgeless")]
    #[graphql(name = "edgeless")]
    Edgeless,

    /// Tomo: Tomo is an all-in-one Web3 social wallet designed to bring the mass adoption of crypto.
    /// It allows users to log in effortlessly with their social accounts and supports multiple chains,
    /// with a particular emphasis on bringing Bitcoin into Ethereum L2, Solana and Cosmos ecosystems.
    /// https://docs.tomo.inc/
    #[strum(serialize = "tomo")]
    #[serde(rename = "tomo")]
    #[graphql(name = "tomo")]
    Tomo,

    /// AILayer: An innovative Bitcoin Layer2 solution,
    /// crafted with a focus on AI-driven modular construction.
    /// https://ailayer.xyz/
    #[strum(serialize = "ailayer")]
    #[serde(rename = "ailayer")]
    #[graphql(name = "ailayer")]
    Ailayer,

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
