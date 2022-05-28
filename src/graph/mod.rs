mod edge;
mod tests;
mod vertex;

use crate::{config::C, error::Error};
use aragog::{schema::DatabaseSchema, AuthMode, DatabaseConnection, OperationOptions};
use serde::Deserialize;

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
