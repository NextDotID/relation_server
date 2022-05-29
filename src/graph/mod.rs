pub mod edge;
mod tests;
pub mod vertex;

use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseConnection, OperationOptions};
pub use edge::Edge;
use serde::Deserialize;
pub use vertex::Vertex;

// TODO: move this under `vertex/`
#[derive(Deserialize, Debug)]
pub struct CryptoIdentity {
    pub uuid: String,
    pub public_key: String,
    pub algorithm: String,
    pub curve: String,
    pub created_at: u128,
}

// TODO: move this under `edge/`
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
