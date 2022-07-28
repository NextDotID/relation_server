use aragog::{
    query::{Comparison, Filter, QueryResult},
    DatabaseConnection, DatabaseRecord, EdgeRecord, Record,
};
use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::vertex::{contract::Chain, Contract, Identity},
    upstream::{DataFetcher, DataSource},
    util::naive_now,
};

use super::Edge;

/// HODL™
#[derive(Clone, Deserialize, Serialize, Record, Debug)]
#[collection_name = "Holds"]
pub struct Hold {
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
pub struct HoldRecord(DatabaseRecord<EdgeRecord<Hold>>);

impl std::ops::Deref for HoldRecord {
    type Target = DatabaseRecord<EdgeRecord<Hold>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HoldRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<EdgeRecord<Hold>>> for HoldRecord {
    fn from(record: DatabaseRecord<EdgeRecord<Hold>>) -> Self {
        Self(record)
    }
}

impl Hold {
    /// Find a hold record by from, to and NFT_ID.
    pub async fn find_by_from_to_id(
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Contract>,
        id: &str,
    ) -> Result<Option<HoldRecord>, Error> {
        let filter = Filter::new(Comparison::field("_from").equals_str(from.id()))
            .and(Comparison::field("_to").equals_str(to.id()))
            .and(Comparison::field("id").equals_str(id));
        let query = EdgeRecord::<Hold>::query().filter(filter);
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
    ) -> Result<Option<HoldRecord>, Error> {
        // TODO: Really should merge these 2 queries into one.
        let contract = Contract::find_by_chain_address(db, chain, address).await?;
        if contract.is_none() {
            return Ok(None);
        }

        let filter = Filter::new(Comparison::field("id").equals_str(id))
            .and(Comparison::field("_to").equals_str(contract.unwrap().id()));
        let query = EdgeRecord::<Hold>::query().filter(filter);
        let result: QueryResult<EdgeRecord<Self>> = query.call(db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().clone().into()))
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Contract, HoldRecord> for Hold {
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid> {
        Some(self.uuid)
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Contract>,
    ) -> Result<HoldRecord, Error> {
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
    ) -> Result<Option<HoldRecord>, Error> {
        let result: QueryResult<EdgeRecord<Hold>> = EdgeRecord::<Hold>::query()
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

#[cfg(test)]
mod tests {
    use crate::{graph::new_db_connection, util::naive_now};
    use fake::{Dummy, Fake, Faker};

    use super::*;

    impl Dummy<Faker> for Hold {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            Self {
                uuid: Uuid::new_v4(),
                source: DataSource::Unknown,
                transaction: config.fake(),
                id: config.fake(),
                created_at: Some(naive_now()),
                updated_at: naive_now(),
                fetcher: Default::default(),
            }
        }
    }

    #[tokio::test]
    async fn test_find_by_id_chain_address() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let identity = Identity::create_dummy(&db).await?;
        let contract = Contract::create_dummy(&db).await?;
        let hold: Hold = Faker.fake();
        let hold_record = hold.connect(&db, &identity, &contract).await?;
        let found =
            Hold::find_by_id_chain_address(&db, &hold.id, &contract.chain, &contract.address)
                .await
                .expect("Should find a Hold record without error")
                .expect("Should find a Hold record, not Empty");
        assert_eq!(found.key(), hold_record.key());

        Ok(())
    }
}
