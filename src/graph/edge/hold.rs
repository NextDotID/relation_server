use aragog::{
    query::{Comparison, Filter, QueryResult},
    DatabaseConnection, DatabaseRecord, EdgeRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::vertex::{Contract, Identity},
    upstream::DataSource,
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
    pub async fn find_by_from_to_source(
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Contract>,
        source: &DataSource,
    ) -> Result<Option<HoldRecord>, Error> {
        let filter = Filter::new(Comparison::field("_from").equals_str(from.id()))
            .and(Comparison::field("_to").equals_str(to.id()))
            .and(Comparison::field("source").equals_str(source));
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
        let found = Self::find_by_from_to_source(db, from, to, &self.source).await?;
        match found {
            Some(edge) => Ok(edge.into()),
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
}

#[cfg(test)]
mod tests {
    use crate::util::naive_now;
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
            }
        }
    }
}
