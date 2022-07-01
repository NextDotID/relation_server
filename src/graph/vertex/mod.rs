mod identity;
pub mod nft;
// mod crypto_identity;

use aragog::{DatabaseConnection, Record};
use async_trait::async_trait;
pub use identity::{Identity, IdentityRecord};
pub use nft::{NFTRecord, NFT};
use uuid::Uuid;

use crate::error::Error;

/// All `Vertex` records.
#[async_trait]
pub trait Vertex<RecordType>
where
    Self: Sized + Record,
{
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid>;

    /// Create or update a vertex.
    async fn create_or_update(&self, db: &DatabaseConnection) -> Result<RecordType, Error>;

    /// Find a vertex by UUID.
    async fn find_by_uuid(db: &DatabaseConnection, uuid: Uuid)
                          -> Result<Option<RecordType>, Error>;

    /// Judge if this record is outdated.
    fn is_outdated(&self) -> bool;
}
