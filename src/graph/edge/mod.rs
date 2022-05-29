mod proof;
// mod pubkey_derivation;

use aragog::{DatabaseConnection, EdgeRecord, Record};
use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Error;

/// All `Edge` records.
#[async_trait]
pub trait Edge
where
    Self: Sized + Record,
{
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid>;

    /// Find an edge by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<EdgeRecord<Self>>, Error>;
}
