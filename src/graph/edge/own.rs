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
    util::naive_now,
};

use super::Edge;

#[derive(Clone, Deserialize, Serialize, Record, Debug)]
#[collection_name = "Owns"]
pub struct Own {
    /// UUID of this record.
    pub uuid: Uuid,
    /// Data source (upstream) which provides this info.
    /// Theoretically, Contract info should only be fetched by chain's RPC server,
    /// but in practice, we still rely on third-party cache / snapshot service.
    pub source: DataSource,
    /// Transaction info of this connection.
    /// i.e. in which `tx` the Contract is transferred / minted.
    /// In most case, it is a `"0xVERY_LONG_HEXSTRING"`.
    /// Maybe this is not provided by `source`, so we set it as `Option<>` here.
    pub transaction: Option<String>,
    /// Token ID in contract. Basiclly `uint256.to_string()`.
    pub token_id: String,
    /// When this connection is built.
    /// if it doesn't provide by `source`, use fetch time instead
    pub connected_at: NaiveDateTime,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct OwnRecord(DatabaseRecord<EdgeRecord<Own>>);

impl std::ops::Deref for OwnRecord {
    type Target = DatabaseRecord<EdgeRecord<Own>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OwnRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<EdgeRecord<Own>>> for OwnRecord {
    fn from(record: DatabaseRecord<EdgeRecord<Own>>) -> Self {
        Self(record)
    }
}

impl Own {
    pub async fn find_by_from_to(
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Contract>,
    ) -> Result<Option<OwnRecord>, Error> {
        let filter = Filter::new(Comparison::field("_from").equals_str(from.id()))
            .and(Comparison::field("_to").equals_str(to.id()));
        let query = EdgeRecord::<Own>::query().filter(filter);
        let result: QueryResult<EdgeRecord<Self>> = query.call(db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().clone().into()))
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Contract, OwnRecord> for Own {
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
    ) -> Result<OwnRecord, Error> {
        let found = Self::find_by_from_to(db, from, to).await?;
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
    ) -> Result<Option<OwnRecord>, Error> {
        let result: QueryResult<EdgeRecord<Own>> = EdgeRecord::<Own>::query()
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

    impl Dummy<Faker> for Own {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            Self {
                uuid: Uuid::new_v4(),
                source: DataSource::Unknown,
                transaction: config.fake(),
                token_id: config.fake(),
                connected_at: naive_now(),
            }
        }
    }
}
