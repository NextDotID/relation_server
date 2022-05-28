mod edge;
mod tests;
mod vertex;

use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseConnection, DatabaseRecord, OperationOptions, Record};
use async_trait::async_trait;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct CryptoIdentity {
    pub uuid: String,
    pub public_key: String,
    pub algorithm: String,
    pub curve: String,
    pub created_at: u128,
}

#[derive(Deserialize, Debug)]
pub struct PubKeyDerivation {
    pub uuid: String,
    pub method: String,
}

/// Create a database connection instance.
pub async fn new_db_connection() -> Result<DatabaseConnection, Error> {
    let connection = DatabaseConnection::builder()
        .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
        .with_auth_mode(AuthMode::Basic)
        .with_operation_options(OperationOptions::default())
        .with_schema_path(&C.db.schema_path)
        .apply_schema() // TODO: run it only on cold start of the server, or manually triggered it
        .build()
        .await?;
    Ok(connection)
}

/// All `Vertex` records.
#[async_trait]
trait Vertex
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
    async fn find_by_uuid(db: &DatabaseConnection, uuid: Uuid) -> Result<Option<Self>, Error>;

    /// Traverse neighbours.
    /// TODO: wrong returning type: should be Edges.
    async fn neighbours(&self, db: &DatabaseConnection) -> Result<Vec<Self>, Error>;
}
