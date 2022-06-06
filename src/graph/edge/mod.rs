pub mod proof;
// mod pubkey_derivation;

pub use proof::{Proof, ProofRecord};

use aragog::{DatabaseConnection, DatabaseRecord, EdgeRecord, Record};
use async_trait::async_trait;
use uuid::Uuid;
pub use proof::Proof;

use crate::error::Error;

/// All `Edge` records.
#[async_trait]
pub trait Edge<From, To, RecordType>
where
    Self: Sized + Record,
    From: Sized + Record,
    To: Sized + Record,
{
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid>;

    /// Connect 2 vertex.
    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &DatabaseRecord<From>,
        to: &DatabaseRecord<To>,
    ) -> Result<RecordType, Error>;

    /// Find an edge by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: &Uuid,
    ) -> Result<Option<RecordType>, Error>;
}
