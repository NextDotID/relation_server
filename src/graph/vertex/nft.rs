use aragog::{DatabaseRecord, Record, DatabaseConnection};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{graph::Vertex, error::Error};

#[derive(Clone, Serialize, Deserialize)]
pub enum Chain {
    Ethereum,
    Rinkeby,
    Ropsten,
    Kovan,
    /// BSC
    BinanceSmartChain,
    Polygon,
    PolygonTestnet,
    /// Solana
    Solana,
    /// Conflux eSpace
    ConfluxESpace,
}
impl Default for Chain {
    fn default() -> Self {
        Chain::Ethereum
    }
}

/// Internal chain implementation / framework.
#[derive(Clone, Serialize, Deserialize)]
pub enum ChainType {
    /// EVM (with chain ID)
    EVM(u128),
    Solana,
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
            BinanceSmartChain => ChainType::EVM(56),
            Polygon => ChainType::EVM(137),
            PolygonTestnet => ChainType::EVM(80001),
            Solana => ChainType::Solana,
            ConfluxESpace => ChainType::EVM(71),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NFTCategory {
    ENS,
}
impl Default for NFTCategory {
    fn default() -> Self {
        NFTCategory::ENS
    }
}

/// NFT
#[derive(Clone, Serialize, Deserialize, Record)]
pub struct NFT {
    /// UUID of this record
    pub uuid: Uuid,
    /// Which NFT is this?
    pub category: NFTCategory,
    /// Contract address
    pub contract: String,
    /// Token ID in contract. Basiclly `uint256.to_string()`.
    pub id: String,
    /// On which chain?
    pub chain: Chain,
    /// Token symbol
    pub symbol: Option<String>,
    /// When this data is fetched by RelationService.
    pub fetched_at: NaiveDateTime,
}

// impl Default for NFT {
// }

#[async_trait::async_trait]
impl Vertex<NFTRecord> for NFT {
    fn uuid(&self) -> Option<uuid::Uuid> {
        Some(self.uuid)
    }

    /// Create or update an NFT.
    async fn create_or_update(&self, db: &DatabaseConnection) -> Result<NFTRecord, Error> {
        todo!()
    }

    /// Find an NFT by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<NFTRecord>, Error> {
        todo!()
    }

    /// What other NFTs does this NFT's owner has?
    async fn neighbors(&self, db: &DatabaseConnection) -> Result<Vec<NFTRecord>, Error> {
        todo!()
    }
}

pub type NFTRecord = DatabaseRecord<NFT>;
