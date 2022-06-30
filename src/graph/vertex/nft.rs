use crate::{error::Error, graph::Vertex, util::naive_now};
use aragog::{
    query::{Comparison, Filter},
    DatabaseConnection, DatabaseRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, Display, PartialEq, EnumString)]
pub enum Chain {
    #[strum(serialize = "ethereum")]
    #[serde(rename = "ethereum")]
    Ethereum,
    #[serde(rename = "rinkeby")]
    #[strum(serialize = "rinkeby")]
    Rinkeby,
    #[serde(rename = "ropsten")]
    #[strum(serialize = "ropsten")]
    Ropsten,
    #[serde(rename = "kovan")]
    #[strum(serialize = "kovan")]
    Kovan,
    #[serde(rename = "bsc")]
    #[strum(serialize = "bsc")]
    BinanceSmartChain,
    #[serde(rename = "polygon")]
    #[strum(serialize = "polygon")]
    Polygon,
    #[serde(rename = "polygon_testnet")]
    #[strum(serialize = "polygon_testnet")]
    PolygonTestnet,
    #[serde(rename = "solana")]
    #[strum(serialize = "solana")]
    Solana,
    #[serde(rename = "conflux_espace")]
    #[strum(serialize = "conflux_espace")]
    ConfluxESpace,
}
impl Default for Chain {
    fn default() -> Self {
        Chain::Ethereum
    }
}

/// Internal chain implementation / framework.
#[derive(Clone, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Serialize, Deserialize, EnumString, Display, Debug)]
pub enum NFTCategory {
    #[strum(serialize = "ENS")]
    #[serde(rename = "ENS")]
    ENS,

    #[strum(serialize = "ERC721")]
    #[serde(rename = "ERC721")]
    ERC721,

    #[strum(serialize = "ERC1155")]
    #[serde(rename = "ERC1155")]
    ERC1155,

    #[strum(serialize = "POAP")]
    #[serde(rename = "POAP")]
    POAP,
}
impl Default for NFTCategory {
    fn default() -> Self {
        NFTCategory::ENS
    }
}

/// NFT
#[derive(Clone, Serialize, Deserialize, Record, Debug)]
#[collection_name = "NFTs"]
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

impl Default for NFT {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            category: Default::default(),
            contract: Default::default(),
            id: Default::default(),
            chain: Default::default(),
            symbol: Default::default(),
            fetched_at: naive_now(),
        }
    }
}

impl NFT {
    async fn find_by_chain_contract_id(
        db: &DatabaseConnection,
        chain: &Chain,
        contract: &String,
        id: &String,
    ) -> Result<Option<NFTRecord>, Error> {
        let query = Self::query().filter(
            Filter::new(Comparison::field("chain").equals_str(chain))
                .and(Comparison::field("contract").equals_str(contract))
                .and(Comparison::field("id").equals_str(id)),
        );
        let result = Self::get(&query, db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().to_owned().into()))
        }
    }
}

#[async_trait::async_trait]
impl Vertex<NFTRecord> for NFT {
    fn uuid(&self) -> Option<uuid::Uuid> {
        Some(self.uuid)
    }

    /// Create or update an NFT.
    async fn create_or_update(&self, db: &DatabaseConnection) -> Result<NFTRecord, Error> {
        let found =
            Self::find_by_chain_contract_id(db, &self.chain, &self.contract, &self.id).await?;
        match found {
            None => {
                let mut to_be_created = self.clone();
                to_be_created.fetched_at = naive_now();
                let created = DatabaseRecord::create(to_be_created, db).await?;
                Ok(created.into())
            }
            Some(mut found) => {
                found.fetched_at = naive_now();
                found.symbol = self.symbol.clone();
                found.save(db).await?;
                Ok(found.into())
            }
        }
    }

    /// Find an NFT by UUID.
    async fn find_by_uuid(db: &DatabaseConnection, uuid: Uuid) -> Result<Option<NFTRecord>, Error> {
        let query = NFT::query().filter(Comparison::field("uuid").equals_str(uuid).into());
        let query_result = NFT::get(&query, db).await?;
        if query_result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(query_result.first().unwrap().to_owned().into()))
        }
    }

    /// What other NFTs does this NFT's owner has?
    async fn neighbors(&self, db: &DatabaseConnection) -> Result<Vec<NFTRecord>, Error> {
        todo!()
    }
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct NFTRecord(DatabaseRecord<NFT>);

impl std::ops::Deref for NFTRecord {
    type Target = DatabaseRecord<NFT>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NFTRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<NFT>> for NFTRecord {
    fn from(record: DatabaseRecord<NFT>) -> Self {
        Self(record)
    }
}
