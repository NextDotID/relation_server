use crate::{error::Error, graph::vertex::Vertex, upstream::Platform, util::naive_now};
use async_trait::async_trait;

use aragog::{
    query::{Comparison, Filter},
    DatabaseConnection, DatabaseRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, Record)]
#[collection_name = "Identities"]
pub struct Identity {
    pub uuid: Option<Uuid>,
    pub platform: Platform,
    /// Usually username.
    pub identity: String,
    /// Usually user-friendly screen name.
    pub display_name: String,
    /// URL to target identity profile page (if any).
    pub profile_url: Option<String>,
    /// URL to avatar (if any).
    pub avatar_url: Option<String>,
    /// Account / identity creation time on target platform.
    pub created_at: Option<NaiveDateTime>,
    /// When this Identity is added into this database.
    pub added_at: NaiveDateTime,
    /// When it is updated
    pub updated_at: NaiveDateTime,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            uuid: None,
            platform: Platform::Twitter,
            identity: Default::default(),
            display_name: Default::default(),
            profile_url: None,
            avatar_url: None,
            created_at: None,
            added_at: naive_now(),
            updated_at: naive_now(),
        }
    }
}

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Identity {
    /// Find record by given platform and identity.
    pub async fn find_by_platform_identity(
        db: &DatabaseConnection,
        platform: &Platform,
        identity: &str,
    ) -> Result<Option<DatabaseRecord<Self>>, Error> {
        let query = Self::query().filter(
            Filter::new(Comparison::field("platform").equals_str(platform))
                .and(Comparison::field("identity").equals_str(identity)),
        );
        let query_result = Self::get(query, db).await?;

        if query_result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(query_result.first().unwrap().to_owned()))
        }
    }
}

#[async_trait]
impl Vertex for Identity {
    fn uuid(&self) -> Option<Uuid> {
        self.uuid
    }

    /// Do create / update side-effect.
    /// Used by upstream crawler.
    async fn create_or_update(
        &self,
        db: &DatabaseConnection,
    ) -> Result<DatabaseRecord<Self>, Error> {
        // Find first
        let found = Self::find_by_platform_identity(db, &self.platform, &self.identity).await?;
        match found {
            None => {
                // Not found. Create it.
                let mut to_be_created = self.clone();
                to_be_created.uuid = to_be_created.uuid.or(Some(Uuid::new_v4()));
                to_be_created.added_at = naive_now();
                to_be_created.updated_at = naive_now();
                let created = DatabaseRecord::create(to_be_created, db).await?;
                Ok(created)
            }
            Some(mut found) => {
                // Found. Update it.
                println!("UUID: {:?}", found.uuid);
                found.display_name = self.display_name.clone();
                found.profile_url = self.profile_url.clone();
                found.avatar_url = self.avatar_url.clone();
                found.created_at = self.created_at;
                found.updated_at = naive_now();

                found.save(db).await?;
                Ok(found)
            }
        }
    }

    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<DatabaseRecord<Identity>>, Error> {
        let query = Identity::query().filter(Comparison::field("uuid").equals_str(uuid).into());
        let query_result = Identity::get(query, db).await?;
        if query_result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(query_result.first().unwrap().to_owned()))
        }
    }

    async fn neighbors(&self, db: &DatabaseConnection) -> Result<Vec<Identity>, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use aragog::{DatabaseConnection, DatabaseRecord};
    use fake::{Dummy, Fake, Faker};
    use uuid::Uuid;

    use super::Identity;
    use crate::{
        error::Error,
        graph::{new_db_connection, Vertex},
        upstream::Platform,
        util::naive_now,
    };

    impl Identity {
        /// Create test dummy data in database.
        pub async fn create_dummy(
            db: &DatabaseConnection,
        ) -> Result<DatabaseRecord<Identity>, Error> {
            let identity: Identity = Faker.fake();
            identity.create_or_update(db).await
        }
    }

    impl Dummy<Faker> for Identity {
        fn dummy_with_rng<R: rand::Rng + ?Sized>(config: &Faker, _rng: &mut R) -> Self {
            Self {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Twitter,
                identity: config.fake(),
                display_name: config.fake(),
                profile_url: Some(config.fake()),
                avatar_url: Some(config.fake()),
                created_at: Some(config.fake()),
                added_at: naive_now(),
                updated_at: naive_now(),
            }
        }
    }

    #[tokio::test]
    async fn test_create() -> Result<(), Error> {
        let identity: Identity = Faker.fake();
        let db = new_db_connection().await?;
        let result = identity.create_or_update(&db).await?;
        assert!(result.uuid.is_some());
        assert!(result.key().len() > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_update() -> Result<(), Error> {
        let db = new_db_connection().await?;

        let mut identity: Identity = Faker.fake();
        let created = identity.create_or_update(&db).await?;

        // Change some of data
        identity.avatar_url = Some(Faker.fake());
        identity.profile_url = Some(Faker.fake());
        let updated = identity.create_or_update(&db).await?;

        assert_eq!(created.uuid, updated.uuid);
        assert_eq!(created.key(), updated.key());
        assert_ne!(created.avatar_url, updated.avatar_url);
        assert_ne!(created.profile_url, updated.profile_url);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_uuid() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let created = Identity::create_dummy(&db).await?;
        let uuid = created.uuid.unwrap();

        let found = Identity::find_by_uuid(&db, uuid).await?;
        assert_eq!(found.unwrap().uuid, created.uuid);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_platform_identity() -> Result<(), Error> {
        let db = new_db_connection().await?;
        let created = Identity::create_dummy(&db).await?;

        let found = Identity::find_by_platform_identity(&db, &created.platform, &created.identity)
            .await?
            .expect("Record not found");
        assert_eq!(found.uuid, created.uuid);

        Ok(())
    }
}
