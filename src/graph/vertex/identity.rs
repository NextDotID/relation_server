use crate::{error::Error, graph::Vertex, upstream::Platform, util::naive_now};
use async_trait::async_trait;

use aragog::{
    query::{Comparison, Filter, QueryResult},
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

#[async_trait]
impl Vertex for Identity {
    fn uuid(&self) -> Option<Uuid> {
        self.uuid
    }

    /// Do create / update side-effect depends on whether there is `uuid`.
    async fn create_or_update(
        &self,
        db: &DatabaseConnection,
    ) -> Result<DatabaseRecord<Self>, Error> {
        match self.uuid {
            None => {
                // Create
                let mut to_be_created = self.clone();
                to_be_created.uuid = Some(Uuid::new_v4());
                to_be_created.added_at = naive_now();
                to_be_created.updated_at = naive_now();
                let created = DatabaseRecord::create(to_be_created, db).await?;
                Ok(created)
            }
            Some(uuid) => {
                // Find first
                let query =
                    Identity::query().filter(Filter::new(Comparison::field("uuid").equals(uuid)));
                let query_result = Identity::get(query, db).await?;
                if query_result.len() == 0 {
                    // Not found. Create it.
                    let mut to_be_created = self.clone();
                    to_be_created.uuid = None;
                    return to_be_created.create_or_update(db).await;
                }
                let found = query_result.first().unwrap();
                Ok(found.to_owned())
            }
        }
    }

    async fn find_by_uuid(db: &DatabaseConnection, uuid: Uuid) -> Result<Option<Identity>, Error> {
        todo!()
    }

    async fn neighbours(&self, db: &DatabaseConnection) -> Result<Vec<Identity>, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::Identity;
    use crate::{
        error::Error,
        graph::{new_db_connection, Vertex},
    };

    #[tokio::test]
    async fn test_create() -> Result<(), Error> {
        let identity = Identity::default();
        let db = new_db_connection().await?;
        let result = identity.create_or_update(&db).await?;
        assert!(result.uuid.is_some());
        assert!(result.key().len() > 0);
        println!("{}", result.key()); // DEBUG

        Ok(())
    }
}
