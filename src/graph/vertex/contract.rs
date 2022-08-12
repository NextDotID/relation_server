use crate::{
    error::Error,
    graph::edge::Hold,
    graph::{ConnectionPool, Vertex},
    util::naive_now,
};
use aragog::{
    query::{Comparison, Filter},
    DatabaseConnection, DatabaseRecord, Record,
};
use arangors_lite::AqlQuery;
use chrono::{Duration, NaiveDateTime};
use dataloader::BatchFn;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::Value};
use std::collections::HashMap;
use strum_macros::{Display, EnumIter, EnumString};
use uuid::Uuid;

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
    #[strum(serialize = "bsc")]
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
            Gnosis => ChainType::EVM(100),
            Arweave => ChainType::Arweave,
            Arbitrum => ChainType::EVM(42161),
            Optimism => ChainType::EVM(10),
            Unknown => todo!(),
        }
    }
}

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
)]
pub enum ContractCategory {
    #[strum(serialize = "ENS")]
    #[serde(rename = "ENS")]
    #[graphql(name = "ENS")]
    ENS,

    #[strum(serialize = "ERC721")]
    #[serde(rename = "ERC721")]
    #[graphql(name = "ERC721")]
    ERC721,

    #[strum(serialize = "ERC1155")]
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

pub struct ContractLoadFn {
    pub pool: ConnectionPool,
}

#[async_trait::async_trait]
impl BatchFn<String, Option<ContractRecord>> for ContractLoadFn {
    async fn load(&mut self, ids: &[String]) -> HashMap<String, Option<ContractRecord>> {
        debug!("Loading contract for: {:?}", ids);
        let contracts = get_contracts(&self.pool, ids.to_vec()).await;
        match contracts {
            Ok(contracts) => contracts,
            // HOLD ON: Not sure if `Err` need to return
            Err(_) => ids.iter().map(|k| (k.to_owned(), None)).collect(),
        }
    }
}

/// It already returns Dataloader friendly output given the NFT IDs.
async fn get_contracts(
    pool: &ConnectionPool,
    ids: Vec<String>,
) -> Result<HashMap<String, Option<ContractRecord>>, Error> {
    let db = pool.db().await?;
    let nft_ids: Vec<Value> = ids.iter().map(|field| json!(field.to_string())).collect();

    let aql = r###"WITH @@edge_collection_name
    FOR d IN @@edge_collection_name
        FILTER d.id IN @nft_ids
        LET v = d._to
        FOR c IN @@collection_name FILTER c._id == v
        RETURN {"id": d.id, "contract": c}"###;

    let aql = AqlQuery::new(aql)
        .bind_var("@edge_collection_name", Hold::COLLECTION_NAME)
        .bind_var("@collection_name", Contract::COLLECTION_NAME)
        .bind_var("nft_ids", nft_ids)
        .batch_size(1)
        .count(false);

    let contracts = db.aql_query::<ToContractRecord>(aql).await;
    match contracts {
        Ok(contents) => {
            let id_contracts_map = contents
                .into_iter()
                .map(|content| (content.id.clone(), Some(content.contract)))
                .collect();

            let dataloader_map = ids.into_iter().fold(
                id_contracts_map,
                |mut map: HashMap<String, Option<ContractRecord>>, id| {
                    map.entry(id).or_insert(None);
                    map
                },
            );

            Ok(dataloader_map)
        }
        Err(e) => Err(Error::ArangoLiteDBError(e)),
    }
}

/// Contract
#[derive(Clone, Serialize, Deserialize, Record, Debug)]
#[collection_name = "Contracts"]
pub struct Contract {
    /// UUID of this record
    pub uuid: Uuid,
    /// What kind of Contract is it?
    pub category: ContractCategory,
    /// Contract address
    pub address: String,
    /// On which chain?
    pub chain: Chain,
    /// Token symbol
    pub symbol: Option<String>,
    /// When this data is fetched by RelationService.
    pub updated_at: NaiveDateTime,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            category: Default::default(),
            address: Default::default(),
            chain: Default::default(),
            symbol: Default::default(),
            updated_at: naive_now(),
        }
    }
}

impl Contract {
    pub async fn find_by_chain_address(
        db: &DatabaseConnection,
        chain: &Chain,
        address: &str,
    ) -> Result<Option<ContractRecord>, Error> {
        let query = Self::query().filter(
            Filter::new(Comparison::field("chain").equals_str(chain))
                .and(Comparison::field("address").equals_str(address)),
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
impl Vertex<ContractRecord> for Contract {
    fn uuid(&self) -> Option<uuid::Uuid> {
        Some(self.uuid)
    }

    /// Create or update an Contract info by (chain, contract, nft_id).
    async fn create_or_update(&self, db: &DatabaseConnection) -> Result<ContractRecord, Error> {
        let found = Self::find_by_chain_address(db, &self.chain, &self.address).await?;
        match found {
            None => {
                let mut to_be_created = self.clone();
                to_be_created.updated_at = naive_now();
                let created = DatabaseRecord::create(to_be_created, db).await?;
                Ok(created.into())
            }
            Some(mut found) => {
                found.updated_at = naive_now();
                found.symbol = self.symbol.clone();
                found.save(db).await?;
                Ok(found)
            }
        }
    }

    /// Find an Contract by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<ContractRecord>, Error> {
        let query = Contract::query().filter(Comparison::field("uuid").equals_str(uuid).into());
        let query_result = Contract::get(&query, db).await?;
        if query_result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(query_result.first().unwrap().to_owned().into()))
        }
    }

    /// Outdated in 1 hour
    fn is_outdated(&self) -> bool {
        let outdated_in = Duration::hours(1);
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct ContractRecord(pub DatabaseRecord<Contract>);

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct ToContractRecord {
    /// NFT_ID of ENS is a hash of domain. So domain can be used as NFT_ID.
    pub id: String,
    /// Account / identity Holds NFT -> Contract
    pub contract: ContractRecord,
}

impl std::ops::Deref for ContractRecord {
    type Target = DatabaseRecord<Contract>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ContractRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<Contract>> for ContractRecord {
    fn from(record: DatabaseRecord<Contract>) -> Self {
        Self(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::new_connection_pool;
    use crate::graph::new_db_connection;
    use fake::{Dummy, Fake, Faker};

    impl Contract {
        pub async fn create_dummy(db: &DatabaseConnection) -> Result<ContractRecord, Error> {
            let nft: Contract = Faker.fake();
            nft.create_or_update(db).await
        }
    }

    impl Dummy<Faker> for Contract {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            let mut nft = Contract::default();
            nft.category = ContractCategory::ENS;
            nft.chain = Chain::Ethereum;
            nft.address = config.fake();
            nft.symbol = Some("ENS".into());

            nft
        }
    }

    #[tokio::test]
    async fn test_creation() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let created = Contract::create_dummy(&db).await?;
        assert!(!created.key().is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_contract_find_by_chain_address() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let created = Contract::create_dummy(&db).await?;
        let found = Contract::find_by_chain_address(&db, &created.chain, &created.address)
            .await?
            .expect("contract should be found");
        assert_eq!(found.key(), created.key());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_contracts_hashmap() -> Result<(), Error> {
        let pool = new_connection_pool().await;
        let ids = vec![
            String::from("2NOea6D9n8T8fQf464L"),
            String::from("lJTcEp2"),
            String::from("fake"),
        ];
        let result = get_contracts(&pool, ids).await;
        println!("{:#?}", result);
        Ok(())
    }
}
