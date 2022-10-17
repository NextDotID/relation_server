pub mod contract;
mod identity;
// mod crypto_identity;

use crate::graph::ConnectionPool;
use crate::upstream::DataSource;
use aragog::{DatabaseConnection, Record};
use async_trait::async_trait;
pub use contract::{Contract, ContractRecord};

pub use identity::{
    FromToLoadFn, Identity, IdentityLoadFn, IdentityRecord, IdentityWithSource, JsonFromToLoadFn,
};
use uuid::Uuid;

use crate::error::Error;

pub fn vec_string_to_vec_datasource(vec_string: Vec<String>) -> Result<Vec<DataSource>, Error> {
    let datasource_result: Result<Vec<DataSource>, _> = vec_string
        .into_iter()
        .map(|p_string| p_string.parse())
        .collect();
    Ok(datasource_result?)
}

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
// #[maybe_async::maybe_async]
#[async_trait]
pub trait Neighbor<EdgeType> {
    async fn neighbors(
        &self,
        pool: &ConnectionPool,
        depth: u16,
    ) -> Result<Vec<IdentityWithSource>, Error>;

    async fn neighbors_with_traversal(
        &self,
        pool: &ConnectionPool,
        depth: u16,
    ) -> Result<Vec<EdgeType>, Error>;
}
