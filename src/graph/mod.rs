pub mod edge;
mod tests;
pub mod vertex;

use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseConnection, OperationOptions};
pub use edge::Edge;
use serde::Deserialize;
pub use vertex::Vertex;

use self::{
    edge::{Hold, Proof},
    vertex::{Contract, Identity},
};

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
        .build()
        .await?;
    Ok(connection)
}

pub async fn create_identity_to_contract_record(
    db: &DatabaseConnection,
    from: &Identity,
    to: &Contract,
    hold: &Hold,
) -> Result<(), Error> {
    let from_record = from.create_or_update(db).await?;
    let to_record = to.create_or_update(db).await?;
    hold.connect(db, &from_record, &to_record).await?;
    Ok(())
}

pub async fn create_identity_to_identity_record(
    db: &DatabaseConnection,
    from: &Identity,
    to: &Identity,
    proof: &Proof,
) -> Result<(), Error> {
    let from_record = from.create_or_update(db).await?;
    let to_record = to.create_or_update(db).await?;
    proof.connect(db, &from_record, &to_record).await?;
    Ok(())
}
