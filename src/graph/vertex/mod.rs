mod identity;
// mod crypto_identity;

use aragog::{DatabaseConnection, DatabaseRecord, Record};
use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Error;

/// All `Vertex` records.
#[async_trait]
pub trait Vertex
where
    Self: Sized + Record,
{
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid>;

    /// Create or update a vertex.
    async fn create_or_update(
        &self,
        db: &DatabaseConnection,
    ) -> Result<DatabaseRecord<Self>, Error>;

    /// Find a vertex by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<DatabaseRecord<Self>>, Error>;

    /// Traverse neighbors.
    async fn neighbors(&self, db: &DatabaseConnection) -> Result<Vec<Self>, Error>;
}
