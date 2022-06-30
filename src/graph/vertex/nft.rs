use crate::{
    error::Error,
    graph::{
        vertex::identity::{Identity, IdentityRecord},
        Vertex,
    },
    util::naive_now,
};
use aragog::{
    query::{Comparison, Filter, QueryResult},
    DatabaseConnection, DatabaseRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumIter};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, Display, PartialEq, EnumString, EnumIter)]
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

#[derive(Clone, Serialize, Deserialize, Debug, EnumString, Display)]
pub enum NFTCategory {
    #[serde(rename = "unknown")]
    #[strum(serialize = "unknown")]
    Unknown,
    #[serde(rename = "ens")]
    #[strum(serialize = "ens")]
    ENS,
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
    pub async fn find_by_chain_contract_id(
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

    /// Create or update an NFT info by (chain, contract, nft_id).
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
    async fn neighbors(&self, _db: &DatabaseConnection) -> Result<Vec<NFTRecord>, Error> {
        Ok(vec![]) // TODO: delete this in trait. Should be implemented in TRecord.
    }
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct NFTRecord(pub DatabaseRecord<NFT>);

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

impl NFTRecord {
    /// Which wallet (`Identity`) does this NFT belong to?
    pub async fn belongs_to(&self, db: &DatabaseConnection) -> Result<Option<IdentityRecord>, Error> {
        let query = self.inbound_query(1, 1, "Owns");

        let result: QueryResult<Identity> = Identity::get(&query, db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().to_owned().into()))
        }
    }

    /// What other NFTs does this NFT's owner has?
    pub async fn neighbors(&self, db: &DatabaseConnection) -> Result<Vec<NFTRecord>, Error> {
        let owner = self.belongs_to(db).await?;
        if owner.is_none() {
            return Ok(vec![]);
        }

        let query = owner.unwrap().outbound_query(1, 2, "Owns");
        let result: QueryResult<NFT> = NFT::get(&query, db).await?;
        if result.len() == 0 {
            Ok(vec![]) // Empty result
        } else {
            Ok(result.iter().map(|r| r.to_owned().into()).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use fake::{Dummy, Faker, Fake};

    use crate::graph::{new_db_connection, edge::Own};

    use super::*;

    impl NFT {
        pub async fn create_dummy(db: &DatabaseConnection) -> Result<NFTRecord, Error> {
            let nft: NFT = Faker.fake();
            Ok(nft.create_or_update(db).await?.into())
        }
    }

    impl Dummy<Faker> for NFT {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            let mut nft = NFT::default();
            nft.category = NFTCategory::ENS;
            nft.chain = Chain::Ethereum;
            nft.contract = config.fake();
            nft.id = config.fake();
            nft.symbol = Some("ENS".into());

            nft
        }
    }

    #[tokio::test]
    async fn test_creation() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let created = NFT::create_dummy(&db).await?;
        assert!(created.key().len() > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_belongs_to() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let nft = NFT::create_dummy(&db).await?;
        let identity = Identity::create_dummy(&db).await?;
        let own: Own = Faker.fake();
        DatabaseRecord::link(&identity, &nft, &db, own).await?;
        let identity_found = nft.belongs_to(&db).await?.expect("Connection not found");
        assert_eq!(identity.uuid, identity_found.uuid);

        Ok(())
    }

    #[tokio::test]
    async fn test_neighbors() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let identity = Identity::create_dummy(&db).await?;
        // Create 2 Identity -> NFT connections
        let nft1 = NFT::create_dummy(&db).await?;
        let own1: Own = Faker.fake();
        DatabaseRecord::link(&identity, &nft1, &db, own1).await?;
        let nft2 = NFT::create_dummy(&db).await?;
        let own2: Own = Faker.fake();
        DatabaseRecord::link(&identity, &nft2, &db, own2).await?;

        let neighbors = nft1.neighbors(&db).await?;
        assert_eq!(2, neighbors.len());

        assert!(neighbors.iter().all(|nft| (nft.uuid == nft1.uuid) || (nft.uuid == nft2.uuid)));
        Ok(())
    }
}
