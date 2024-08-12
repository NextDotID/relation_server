use crate::upstream::Platform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;
use strum_macros::{Display, EnumIter, EnumString};

/// All domain system name.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Display,
    EnumString,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Copy,
    Hash,
    async_graphql::Enum,
)]
pub enum DomainNameSystem {
    /// ENS: Ethereum Name Service
    /// https://ens.domains
    #[strum(serialize = "ens")]
    #[serde(rename = "ens")]
    #[graphql(name = "ens")]
    ENS,

    /// SNS: Solana Name Service
    /// https://www.sns.id
    #[strum(serialize = "sns")]
    #[serde(rename = "sns")]
    #[graphql(name = "sns")]
    SNS,

    /// Dotbit Name Service
    /// https://www.did.id/
    #[strum(serialize = "dotbit")]
    #[serde(rename = "dotbit")]
    #[graphql(name = "dotbit")]
    DotBit,

    /// Lens Protocol
    /// https://api.lens.dev/playground
    #[strum(serialize = "lens")]
    #[serde(rename = "lens")]
    #[graphql(name = "lens")]
    Lens,

    /// UnstoppableDomains
    /// https://unstoppabledomains.com/
    #[strum(serialize = "unstoppabledomains")]
    #[serde(rename = "unstoppabledomains")]
    #[graphql(name = "unstoppabledomains")]
    UnstoppableDomains,

    /// SpaceID: .bnb Name Service
    /// https://api.prd.space.id/
    #[strum(serialize = "space_id")]
    #[serde(rename = "space_id")]
    #[graphql(name = "space_id")]
    SpaceId,

    /// Genome
    #[strum(serialize = "genome")]
    #[serde(rename = "genome")]
    #[graphql(name = "genome")]
    Genome,

    /// https://indexer.crossbell.io/docs
    #[strum(serialize = "crossbell")]
    #[serde(rename = "crossbell")]
    #[graphql(name = "crossbell")]
    Crossbell,

    /// Clusters
    #[strum(serialize = "clusters")]
    #[serde(rename = "clusters")]
    #[graphql(name = "clusters")]
    Clusters,

    /// Zeta Name Service
    #[strum(serialize = "zeta")]
    #[serde(rename = "zeta")]
    #[graphql(name = "zeta")]
    Zeta,

    /// Mode Name Service
    #[strum(serialize = "mode")]
    #[serde(rename = "mode")]
    #[graphql(name = "mode")]
    Mode,

    /// .arb Name Service
    #[strum(serialize = "arb")]
    #[serde(rename = "arb")]
    #[graphql(name = "arb")]
    Arb,

    /// DotTaiko Name Service
    #[strum(serialize = "taiko")]
    #[serde(rename = "taiko")]
    #[graphql(name = "taiko")]
    Taiko,

    /// Mint Name Service
    #[strum(serialize = "mint")]
    #[serde(rename = "mint")]
    #[graphql(name = "mint")]
    Mint,

    /// ZKFair Name Service
    #[strum(serialize = "zkf")]
    #[serde(rename = "zkf")]
    #[graphql(name = "zkf")]
    Zkf,

    /// Manta Name Service
    #[strum(serialize = "manta")]
    #[serde(rename = "manta")]
    #[graphql(name = "manta")]
    Manta,

    /// LightLink Name Service
    #[strum(serialize = "ll")]
    #[serde(rename = "ll")]
    #[graphql(name = "ll")]
    Ll,

    /// Genome Name Service
    #[strum(serialize = "gno")]
    #[serde(rename = "gno")]
    #[graphql(name = "gno")]
    Gno,

    /// Merlin Name Service
    #[strum(serialize = "merlin")]
    #[serde(rename = "merlin")]
    #[graphql(name = "merlin")]
    Merlin,

    /// PancakeSwap Name Service
    #[strum(serialize = "cake")]
    #[serde(rename = "cake")]
    #[graphql(name = "cake")]
    Cake,

    /// ALIENX Name Service
    #[strum(serialize = "alien")]
    #[serde(rename = "alien")]
    #[graphql(name = "alien")]
    Alien,

    /// Floki Name Service
    #[strum(serialize = "floki")]
    #[serde(rename = "floki")]
    #[graphql(name = "floki")]
    Floki,

    /// BurgerCities Name Service
    #[strum(serialize = "burger")]
    #[serde(rename = "burger")]
    #[graphql(name = "burger")]
    Burger,

    /// Tomo Name Service
    #[strum(serialize = "tomo")]
    #[serde(rename = "tomo")]
    #[graphql(name = "tomo")]
    Tomo,

    /// AILayer Name Service
    #[strum(serialize = "ail")]
    #[serde(rename = "ail")]
    #[graphql(name = "ail")]
    Ail,

    #[default]
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    Unknown,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Display,
    EnumString,
    PartialEq,
    Eq,
    EnumIter,
    Default,
    Copy,
    Hash,
    async_graphql::Enum,
)]
pub enum EXT {
    /// ENS: Ethereum Name Service
    /// https://ens.domains
    #[strum(serialize = "eth")]
    #[serde(rename = "eth")]
    #[graphql(name = "eth")]
    Eth,

    /// https://www.sns.id: Solana Name Service
    #[strum(serialize = "sol")]
    #[serde(rename = "sol")]
    #[graphql(name = "sol")]
    Sol,

    /// https://www.did.id/
    #[strum(serialize = "bit")]
    #[serde(rename = "bit")]
    #[graphql(name = "bit")]
    Bit,

    /// https://api.lens.dev/playground
    #[strum(serialize = "lens")]
    #[serde(rename = "lens")]
    #[graphql(name = "lens")]
    Lens,

    /// https://indexer.crossbell.io/docs
    #[strum(serialize = "csb")]
    #[serde(rename = "csb")]
    #[graphql(name = "csb")]
    Csb,

    /// Clusters
    #[strum(serialize = "/")]
    #[serde(rename = "/")]
    #[graphql(name = "/")]
    ClustersRoot,
    #[strum(serialize = "main")]
    #[serde(rename = "main")]
    #[graphql(name = "main")]
    ClustersMain,

    /// UnstoppableDomains
    #[strum(serialize = "x")]
    #[serde(rename = "x")]
    #[graphql(name = "x")]
    X,
    #[strum(serialize = "crypto")]
    #[serde(rename = "crypto")]
    #[graphql(name = "crypto")]
    Crypto,
    #[strum(serialize = "wallet")]
    #[serde(rename = "wallet")]
    #[graphql(name = "wallet")]
    Wallet,
    #[strum(serialize = "blockchain")]
    #[serde(rename = "blockchain")]
    #[graphql(name = "blockchain")]
    Blockchain,
    #[strum(serialize = "bitcoin")]
    #[serde(rename = "bitcoin")]
    #[graphql(name = "bitcoin")]
    Bitcoin,
    #[strum(serialize = "888")]
    #[serde(rename = "888")]
    #[graphql(name = "888")]
    EightEightEight,
    #[strum(serialize = "nft")]
    #[serde(rename = "nft")]
    #[graphql(name = "nft")]
    Nft,
    #[strum(serialize = "dao")]
    #[serde(rename = "dao")]
    #[graphql(name = "dao")]
    Dao,
    #[strum(serialize = "polygon")]
    #[serde(rename = "polygon")]
    #[graphql(name = "polygon")]
    Polygon,
    #[strum(serialize = "unstoppable")]
    #[serde(rename = "unstoppable")]
    #[graphql(name = "unstoppable")]
    Unstoppable,
    #[strum(serialize = "pudgy")]
    #[serde(rename = "pudgy")]
    #[graphql(name = "pudgy")]
    Pudgy,
    #[strum(serialize = "go")]
    #[serde(rename = "go")]
    #[graphql(name = "go")]
    Go,
    #[strum(serialize = "zil")]
    #[serde(rename = "zil")]
    #[graphql(name = "zil")]
    Zil,
    #[strum(serialize = "austin")]
    #[serde(rename = "austin")]
    #[graphql(name = "austin")]
    Austin,
    #[strum(serialize = "raiin")]
    #[serde(rename = "raiin")]
    #[graphql(name = "raiin")]
    Raiin,
    #[strum(serialize = "tball")]
    #[serde(rename = "tball")]
    #[graphql(name = "tball")]
    Tball,

    // Domains (TLDs) using the SPACE ID 3.0 infrastructure
    /// https://api.prd.space.id/
    /// .bnb Name Service
    #[strum(serialize = "bnb")]
    #[serde(rename = "bnb")]
    #[graphql(name = "bnb")]
    Bnb,
    /// Zeta Name Service
    #[strum(serialize = "zeta")]
    #[serde(rename = "zeta")]
    #[graphql(name = "zeta")]
    Zeta,
    /// Mode Name Service
    #[strum(serialize = "mode")]
    #[serde(rename = "mode")]
    #[graphql(name = "mode")]
    Mode,
    /// .arb Name Service (on Arbitrum)
    #[strum(serialize = "arb")]
    #[serde(rename = "arb")]
    #[graphql(name = "arb")]
    Arb,
    /// DotTaiko Name Service (on Scroll)
    #[strum(serialize = "taiko")]
    #[serde(rename = "taiko")]
    #[graphql(name = "taiko")]
    Taiko,
    /// Mint Name Service
    #[strum(serialize = "mint")]
    #[serde(rename = "mint")]
    #[graphql(name = "mint")]
    Mint,
    /// ZKFair Name Service
    #[strum(serialize = "zkf")]
    #[serde(rename = "zkf")]
    #[graphql(name = "zkf")]
    Zkf,
    /// Manta Name Service
    #[strum(serialize = "manta")]
    #[serde(rename = "manta")]
    #[graphql(name = "manta")]
    Manta,
    /// LightLink Name Service
    #[strum(serialize = "ll")]
    #[serde(rename = "ll")]
    #[graphql(name = "ll")]
    Ll,
    /// Genome Name Service
    #[strum(serialize = "gno")]
    #[serde(rename = "gno")]
    #[graphql(name = "gno")]
    Gno,
    /// Merlin Name Service
    #[strum(serialize = "merlin")]
    #[serde(rename = "merlin")]
    #[graphql(name = "merlin")]
    Merlin,
    /// PancakeSwap Name Service (BNB SmartChain)
    #[strum(serialize = "cake")]
    #[serde(rename = "cake")]
    #[graphql(name = "cake")]
    Cake,
    /// ALIENX Name Service
    #[strum(serialize = "alien")]
    #[serde(rename = "alien")]
    #[graphql(name = "alien")]
    Alien,
    /// Floki Name Service (BNB SmartChain)
    #[strum(serialize = "floki")]
    #[serde(rename = "floki")]
    #[graphql(name = "floki")]
    Floki,
    /// BurgerCities Name Service (BNB SmartChain)
    #[strum(serialize = "burger")]
    #[serde(rename = "burger")]
    #[graphql(name = "burger")]
    Burger,
    /// Tomo Name Service
    #[strum(serialize = "tomo")]
    #[serde(rename = "tomo")]
    #[graphql(name = "tomo")]
    Tomo,
    /// AILayer Name Service
    #[strum(serialize = "ail")]
    #[serde(rename = "ail")]
    #[graphql(name = "ail")]
    Ail,

    // Unknown
    #[default]
    #[strum(serialize = "unknown")]
    #[serde(rename = "unknown")]
    #[graphql(name = "unknown")]
    Unknown,
}

pub fn trim_name(name: &str) -> String {
    // Handle Clusters special case: clusters/main
    if let Some(pos) = name.find('/') {
        return name[..pos].to_string();
    }

    // Split by '.' to identify subdomains, names, and suffixes
    let parts: Vec<&str> = name.split('.').collect();

    if parts.len() == 1 {
        // No '.' in name, return it as is
        return parts[0].to_string();
    } else if parts.len() == 2 {
        // Handle case where there's just a name.ext
        let (process_name, suffix) = (parts[0], parts[1]);
        // Check if suffix is in the EXT enum
        if EXT::from_str(suffix).is_ok() {
            return process_name.to_string();
        }
    } else if parts.len() > 2 {
        // Handle case with subdomain, e.g., subdomain.name.ext
        let process_name = parts[1]; // Middle part is the process_name
        let suffix = parts.last().unwrap();
        if EXT::from_str(suffix).is_ok() {
            return process_name.to_string();
        }
    }

    name.to_string()
}

// Extension hashmap initialization
lazy_static! {
    pub static ref EXTENSION: HashMap<Platform, Vec<EXT>> = {
        let mut extension = HashMap::new();
        extension.insert(Platform::ENS, vec![EXT::Eth]); // name.eth
        extension.insert(Platform::SNS, vec![EXT::Sol]);  // name.sol
        extension.insert(Platform::Dotbit, vec![EXT::Bit]); // name.bit
        extension.insert(Platform::Lens, vec![EXT::Lens]); // lens/handle
        extension.insert(Platform::Crossbell, vec![EXT::Csb]); // name.csb or address.csb
        extension.insert(Platform::Clusters, vec![EXT::ClustersRoot, EXT::ClustersMain]); // clusters/ or clusters/main
        extension.insert(Platform::Farcaster, vec![]);
        extension.insert(Platform::Unknown, vec![]);

        extension.insert(Platform::UnstoppableDomains, vec![
            EXT::Crypto,
            EXT::Wallet,
            EXT::Blockchain,
            EXT::Bitcoin,
            EXT::X,
            EXT::EightEightEight,
            EXT::Nft,
            EXT::Dao,
            EXT::Polygon,
            EXT::Unstoppable,
            EXT::Pudgy,
            EXT::Go,
            EXT::Zil,
            EXT::Austin,
            EXT::Raiin,
            EXT::Tball]);

        extension.insert(Platform::SpaceId, vec![
            EXT::Bnb,
            EXT::Cake,
            EXT::Floki,
            EXT::Burger,
            ]);

        extension.insert(Platform::Zeta, vec![EXT::Zeta]);
        extension.insert(Platform::Mode, vec![EXT::Mode]);
        extension.insert(Platform::Arbitrum, vec![EXT::Arb]);
        extension.insert(Platform::Taiko, vec![EXT::Taiko]);
        extension.insert(Platform::Mint, vec![EXT::Mint]);
        extension.insert(Platform::Zkfair, vec![EXT::Zkf]);
        extension.insert(Platform::Manta, vec![EXT::Manta]);
        extension.insert(Platform::Lightlink, vec![EXT::Ll]);
        extension.insert(Platform::Genome, vec![EXT::Gno]);
        extension.insert(Platform::Merlin, vec![EXT::Merlin]);
        extension.insert(Platform::AlienX, vec![EXT::Alien]);
        extension.insert(Platform::Tomo, vec![EXT::Tomo]);
        extension.insert(Platform::Ailayer, vec![EXT::Ail]);

        extension
    };
}

impl From<EXT> for Platform {
    fn from(ext: EXT) -> Self {
        match ext {
            EXT::Eth => Platform::ENS,
            EXT::Sol => Platform::SNS,
            EXT::Bit => Platform::Dotbit,
            EXT::Lens => Platform::Lens,
            EXT::Csb => Platform::Crossbell,

            // UnstoppableDomains extensions
            EXT::X => Platform::UnstoppableDomains,
            EXT::Crypto => Platform::UnstoppableDomains,
            EXT::Wallet => Platform::UnstoppableDomains,
            EXT::Blockchain => Platform::UnstoppableDomains,
            EXT::Bitcoin => Platform::UnstoppableDomains,
            EXT::EightEightEight => Platform::UnstoppableDomains,
            EXT::Nft => Platform::UnstoppableDomains,
            EXT::Dao => Platform::UnstoppableDomains,
            EXT::Polygon => Platform::UnstoppableDomains,
            EXT::Unstoppable => Platform::UnstoppableDomains,
            EXT::Pudgy => Platform::UnstoppableDomains,
            EXT::Go => Platform::UnstoppableDomains,
            EXT::Zil => Platform::UnstoppableDomains,
            EXT::Austin => Platform::UnstoppableDomains,
            EXT::Raiin => Platform::UnstoppableDomains,
            EXT::Tball => Platform::UnstoppableDomains,

            // SpaceID 3.0 extensions
            EXT::Bnb => Platform::SpaceId,
            EXT::Zeta => Platform::Zeta,
            EXT::Mode => Platform::Mode,
            EXT::Arb => Platform::Arbitrum,
            EXT::Taiko => Platform::Taiko,
            EXT::Mint => Platform::Mint,
            EXT::Zkf => Platform::Zkfair,
            EXT::Manta => Platform::Manta,
            EXT::Ll => Platform::Lightlink,
            EXT::Gno => Platform::Genome,
            EXT::Merlin => Platform::Merlin,
            EXT::Cake => Platform::SpaceId,
            EXT::Alien => Platform::AlienX,
            EXT::Floki => Platform::SpaceId,
            EXT::Burger => Platform::SpaceId,
            EXT::Tomo => Platform::Tomo,
            EXT::Ail => Platform::Ailayer,
            _ => Platform::Unknown,
        }
    }
}

impl From<EXT> for DomainNameSystem {
    fn from(ext: EXT) -> Self {
        match ext {
            EXT::Eth => DomainNameSystem::ENS,
            EXT::Sol => DomainNameSystem::SNS,
            EXT::Bit => DomainNameSystem::DotBit,
            EXT::Lens => DomainNameSystem::Lens,
            EXT::Csb => DomainNameSystem::Crossbell,

            // UnstoppableDomains extensions
            EXT::X => DomainNameSystem::UnstoppableDomains,
            EXT::Crypto => DomainNameSystem::UnstoppableDomains,
            EXT::Wallet => DomainNameSystem::UnstoppableDomains,
            EXT::Blockchain => DomainNameSystem::UnstoppableDomains,
            EXT::Bitcoin => DomainNameSystem::UnstoppableDomains,
            EXT::EightEightEight => DomainNameSystem::UnstoppableDomains,
            EXT::Nft => DomainNameSystem::UnstoppableDomains,
            EXT::Dao => DomainNameSystem::UnstoppableDomains,
            EXT::Polygon => DomainNameSystem::UnstoppableDomains,
            EXT::Unstoppable => DomainNameSystem::UnstoppableDomains,
            EXT::Pudgy => DomainNameSystem::UnstoppableDomains,
            EXT::Go => DomainNameSystem::UnstoppableDomains,
            EXT::Zil => DomainNameSystem::UnstoppableDomains,
            EXT::Austin => DomainNameSystem::UnstoppableDomains,
            EXT::Raiin => DomainNameSystem::UnstoppableDomains,
            EXT::Tball => DomainNameSystem::UnstoppableDomains,

            // SpaceID 3.0 extensions
            EXT::Bnb => DomainNameSystem::SpaceId,
            EXT::Zeta => DomainNameSystem::Zeta,
            EXT::Mode => DomainNameSystem::Mode,
            EXT::Arb => DomainNameSystem::Arb,
            EXT::Taiko => DomainNameSystem::Taiko,
            EXT::Mint => DomainNameSystem::Mint,
            EXT::Zkf => DomainNameSystem::Zkf,
            EXT::Manta => DomainNameSystem::Manta,
            EXT::Ll => DomainNameSystem::Ll,
            EXT::Gno => DomainNameSystem::Genome,
            EXT::Merlin => DomainNameSystem::Merlin,
            EXT::Cake => DomainNameSystem::SpaceId,
            EXT::Alien => DomainNameSystem::Alien,
            EXT::Floki => DomainNameSystem::SpaceId,
            EXT::Burger => DomainNameSystem::SpaceId,
            EXT::Tomo => DomainNameSystem::Tomo,
            EXT::Ail => DomainNameSystem::Ail,
            _ => DomainNameSystem::Unknown,
        }
    }
}

impl From<DomainNameSystem> for Platform {
    fn from(domain: DomainNameSystem) -> Self {
        match domain {
            DomainNameSystem::ENS => Platform::ENS,
            DomainNameSystem::SNS => Platform::SNS,
            DomainNameSystem::DotBit => Platform::Dotbit,
            DomainNameSystem::UnstoppableDomains => Platform::UnstoppableDomains,
            DomainNameSystem::Lens => Platform::Lens,
            DomainNameSystem::SpaceId => Platform::SpaceId,
            DomainNameSystem::Genome => Platform::Genome,
            DomainNameSystem::Crossbell => Platform::Crossbell,
            DomainNameSystem::Clusters => Platform::Clusters,
            _ => Platform::Unknown,
        }
    }
}
