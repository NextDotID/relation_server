use aragog::{
    query::{Comparison, Filter, QueryResult},
    DatabaseConnection, DatabaseRecord, EdgeRecord, Record,
};
use arangors_lite::AqlQuery;
use async_graphql::ID;
use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::{
        vertex::{contract::Chain, Contract, Identity},
        ConnectionPool,
    },
    upstream::{DataFetcher, DataSource},
    util::naive_now,
};

use super::Edge;
/// HODL™
#[derive(Clone, Deserialize, Serialize, Record, Debug)]
#[collection_name = "Holds"]
pub struct Hold2 {
    /// UUID of this record.
    pub uuid: Uuid,
    /// Data source (upstream) which provides this info.
    /// Theoretically, Contract info should only be fetched by chain's RPC server,
    /// but in practice, we still rely on third-party cache / snapshot service.
    pub source: DataSource,
    /// Transaction info of this connection.
    /// i.e. in which `tx` the Contract is transferred / minted.
    /// In most case, it is a `"0xVERY_LONG_HEXSTRING"`.
    /// It happens that this info is not provided by `source`, so we treat it as `Option<>`.
    pub transaction: Option<String>,
    /// NFT_ID in contract / ENS domain / anything can be used as an unique ID to specify the held object.
    /// It must be one here.
    /// Tips: NFT_ID of ENS is a hash of domain. So domain can be used as NFT_ID.
    pub id: String,
    /// When the transaction happened. May not be provided by upstream.
    pub created_at: Option<NaiveDateTime>,
    /// When this HODL™ relation is fetched by us RelationService.
    pub updated_at: NaiveDateTime,
    /// Who collects this data.
    /// It works as a "data cleansing" or "proxy" between `source`s and us.
    pub fetcher: DataFetcher,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct HoldRecord2(DatabaseRecord<EdgeRecord<Hold2>>);

impl std::ops::Deref for HoldRecord2 {
    type Target = DatabaseRecord<EdgeRecord<Hold2>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HoldRecord2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<EdgeRecord<Hold2>>> for HoldRecord2 {
    fn from(record: DatabaseRecord<EdgeRecord<Hold2>>) -> Self {
        Self(record)
    }
}

impl Hold2 {
    /// Find a hold record by from, to and NFT_ID.
    pub async fn find_by_from_to_id(
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Identity>,
        id: &str,
    ) -> Result<Option<HoldRecord2>, Error> {
        let filter = Filter::new(Comparison::field("_from").equals_str(from.id()))
            .and(Comparison::field("_to").equals_str(to.id()))
            .and(Comparison::field("id").equals_str(id));
        let query = EdgeRecord::<Hold2>::query().filter(filter);
        let result: QueryResult<EdgeRecord<Self>> = query.call(db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().clone().into()))
        }
    }

    /// Find a hold record by Chain, NFT_ID and NFT Address.
    pub async fn find_by_id_chain_address(
        db: &DatabaseConnection,
        id: &str,
        chain: &Chain,
        address: &str,
    ) -> Result<Option<HoldRecord2>, Error> {
        // TODO: Really should merge these 2 queries into one.
        let contract = Contract::find_by_chain_address(db, chain, address).await?;
        if contract.is_none() {
            return Ok(None);
        }

        let filter = Filter::new(Comparison::field("id").equals_str(id))
            .and(Comparison::field("_to").equals_str(contract.unwrap().id()));
        let query = EdgeRecord::<Hold2>::query().filter(filter);
        let result: QueryResult<EdgeRecord<Self>> = query.call(db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().clone().into()))
        }
    }

    /// Find a hold record by Chain, NFT_ID and NFT Address.
    /// merge these 2 queries into one.
    pub async fn find_by_id_chain_address_merge(
        pool: &ConnectionPool,
        id: &str,
        chain: &Chain,
        address: &str,
    ) -> Result<Option<HoldRecord2>, Error> {
        let db = pool.db().await?;
        let aql_str = r"FOR c IN @@collection_name
            FILTER c.address == @address AND c.chain == @chain
            FOR vertex, edge IN 1..1 INBOUND c GRAPH @graph_name
            FILTER edge.id == @id
            RETURN edge
        ";
        let aql = AqlQuery::new(aql_str)
            .bind_var("@collection_name", Contract::COLLECTION_NAME)
            .bind_var("graph_name", "identities_contracts_graph")
            .bind_var("address", address)
            .bind_var("chain", chain.to_string())
            .bind_var("id", id)
            .batch_size(1)
            .count(false);

        let holds = db.aql_query::<HoldRecord2>(aql).await?;
        if holds.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(holds.first().unwrap().clone().into()))
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Identity, HoldRecord2> for Hold2 {
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid> {
        Some(self.uuid)
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Identity>,
    ) -> Result<HoldRecord2, Error> {
        let found = Self::find_by_from_to_id(db, from, to, &self.id).await?;
        match found {
            Some(edge) => Ok(edge),
            None => Ok(DatabaseRecord::link(from, to, db, self.clone())
                .await?
                .into()),
        }
    }

    /// Find an edge by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: &Uuid,
    ) -> Result<Option<HoldRecord2>, Error> {
        let result: QueryResult<EdgeRecord<Hold2>> = EdgeRecord::<Hold2>::query()
            .filter(Comparison::field("uuid").equals_str(uuid).into())
            .call(db)
            .await?;

        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().to_owned().into()))
        }
    }

    fn is_outdated(&self) -> bool {
        let outdated_in = Duration::hours(8);
        self.updated_at
            .checked_add_signed(outdated_in)
            .unwrap()
            .lt(&naive_now())
    }
}
