pub mod hold;
pub mod proof;
pub mod resolve;
// mod pubkey_derivation;

pub use hold::{Hold, HoldRecord};
pub use proof::{Proof, ProofRecord};
pub use resolve::{Resolve, ResolveRecord};

use aragog::{DatabaseConnection, DatabaseRecord, Record};
use async_trait::async_trait;
use uuid::Uuid;

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
    // TODO after the transfer, connect record need to update
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

    /// Judge if this record is outdated.
    fn is_outdated(&self) -> bool;
}
