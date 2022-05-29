use aragog::{
    query::{Comparison, Filter, QueryResult},
    DatabaseConnection, DatabaseRecord, EdgeRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::{vertex::Identity, Edge},
    upstream::DataSource,
};

/// Edge to connect two `Identity`s.
#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[collection_name = "Proofs"]
pub struct Proof {
    pub uuid: Uuid,
    /// Data source (upstream) which provided this connection.
    pub source: DataSource,
    /// ID of this connection in upstream platform to locate (if any).
    pub record_id: Option<String>,
    /// Connection creation time in upstream platform (if any).
    pub created_at: Option<NaiveDateTime>,
    /// When this connection is fetched by RelationService.
    pub last_fetched_at: NaiveDateTime,
}

impl Proof {
    pub async fn find_by_from_to(
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Identity>,
        source: &DataSource,
        record_id: &Option<String>,
    ) -> Result<Option<DatabaseRecord<EdgeRecord<Self>>>, Error> {
        let mut filter = Filter::new(Comparison::field("_from").equals_str(from.id()))
            .and(Comparison::field("_to").equals_str(to.id()))
            .and(Comparison::field("source").equals_str(source));
        if record_id.is_some() {
            filter =
                filter.and(Comparison::field("record_id").equals_str(record_id.clone().unwrap()));
        }
        let query = EdgeRecord::<Proof>::query().filter(filter);
        let result: QueryResult<EdgeRecord<Self>> = query.call(db).await?;
        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().clone()))
        }
    }
}

#[async_trait::async_trait]
impl Edge<Identity, Identity> for Proof {
    fn uuid(&self) -> Option<Uuid> {
        Some(self.uuid)
    }

    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: &Uuid,
    ) -> Result<Option<DatabaseRecord<EdgeRecord<Self>>>, Error> {
        let result: QueryResult<EdgeRecord<Proof>> = EdgeRecord::<Proof>::query()
            .filter(Comparison::field("uuid").equals_str(uuid).into())
            .call(db)
            .await?;

        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().to_owned()))
        }
    }

    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<Identity>,
    ) -> Result<DatabaseRecord<EdgeRecord<Self>>, Error> {
        // Find first
        let found = Self::find_by_from_to(db, from, to, &self.source, &self.record_id).await?;
        match found {
            // Found. Return it.
            Some(edge) => Ok(edge),
            // Not found. Create it.
            None => Ok(DatabaseRecord::link(from, to, db, self.clone()).await?),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::new_db_connection, util::naive_now};
    use fake::{Dummy, Fake, Faker};

    use super::*;

    impl Dummy<Faker> for Proof {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            Self {
                uuid: Uuid::new_v4(),
                source: DataSource::SybilList,
                record_id: Some(config.fake()),
                created_at: Some(config.fake()),
                last_fetched_at: naive_now(),
            }
        }
    }

    #[tokio::test]
    async fn test_create_and_find() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let from = Identity::create_dummy(&db).await?;
        let to = Identity::create_dummy(&db).await?;
        let connection: Proof = Faker.fake();
        let generated = connection.connect(&db, &from, &to).await?;

        assert_eq!(generated.id_from().clone(), from.id().clone());
        assert_eq!(generated.id_to().clone(), to.id().clone());
        assert_eq!(generated.record_id, connection.record_id);
        assert_eq!(generated.source, connection.source);
        assert_eq!(generated.uuid, connection.uuid);

        let found_by_from_to =
            Proof::find_by_from_to(&db, &from, &to, &connection.source, &connection.record_id)
                .await?
                .unwrap();
        assert_eq!(found_by_from_to.uuid, generated.uuid);

        let found_by_uuid = Proof::find_by_uuid(&db, &generated.uuid).await?.unwrap();
        assert_eq!(found_by_uuid.uuid, generated.uuid);

        Ok(())
    }
}
