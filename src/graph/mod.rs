pub mod arangopool;
pub mod edge;
pub mod pool;
mod tests;
pub mod vertex;
use deadpool::managed::Pool;
use std::collections::HashMap;
use tracing::debug;

use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseConnection, OperationOptions};
use arangors_lite::{
    view::ArangoSearchViewLink, view::ArangoSearchViewPropertiesOptions, view::ViewDescription,
    view::ViewOptions, view::ViewType, Connection, Database,
};

pub use edge::Edge;
use serde::Deserialize;
pub use vertex::Vertex;

use self::{
    edge::{Hold, HoldRecord, Proof},
    vertex::{Contract, ContractRecord, Identity, IdentityRecord},
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

#[derive(Clone)]
pub struct ConnectionPool {
    pub pool: Pool<Connection, Error>,
}

impl ConnectionPool {
    pub async fn db(&self) -> Result<Database, Error> {
        let pool_status = &self.pool.status();
        debug!(
            "Connection pool status: max_size={}, size={}, available={}",
            pool_status.max_size, pool_status.size, pool_status.available
        );
        let connection = &self.pool.get().await;
        match connection {
            Ok(conn) => match conn.db(&C.db.db).await {
                Ok(db) => Ok(db),
                Err(err) => Err(Error::ArangoLiteDBError(err)),
            },
            Err(err) => Err(Error::PoolError(err.to_string())),
        }
    }
}

/// Create connection pool for arangodb
pub async fn new_connection_pool() -> ConnectionPool {
    // HOLD ON: Specify the maximum number(1024)
    // let max_pool_size = num_cpus::get_physical() * 4;
    let max_pool_size = 24;
    let connection_pool = Pool::new(
        pool::ConnectionManager {
            host: C.db.host.to_string(),
            username: C.db.username.to_string(),
            password: C.db.password.to_string(),
            db: C.db.db.to_string(),
        },
        max_pool_size,
    );
    debug!("Creating connection pool(db={})", &C.db.db);
    ConnectionPool {
        pool: connection_pool,
    }
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
) -> Result<(IdentityRecord, ContractRecord, HoldRecord), Error> {
    let from_record = from.create_or_update(db).await?;
    let to_record = to.create_or_update(db).await?;
    let hold_record = hold.connect(db, &from_record, &to_record).await?;
    Ok((from_record, to_record, hold_record))
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

// Create a row database connection instance for arangodb
pub async fn new_raw_db_connection() -> Result<Database, Error> {
    let conn = Connection::establish_basic_auth(&C.db.host, &C.db.username, &C.db.password).await?;
    let db = conn.db(&C.db.db).await?;
    let views: Vec<ViewDescription> = db.list_views().await?;
    let view_name = "relation";
    if views.into_iter().any(|r| r.name == view_name) {
        return Ok(db);
    }

    /* else create_arangosearch_view: [relation] {
        "name":"relation",
        "links":{
            "Identities":{
                "includeAllFields":true,
                "fields":{
                    "identity":{"analyzers":["text_en"]},
                    "platform":{"analyzers":["text_en"]},
                    "display_name":{"analyzers":["text_en"]}
                }
            }
        }
    }*/

    let fields = HashMap::from([
        (
            "identity".to_string(),
            ArangoSearchViewLink::builder()
                .analyzers(vec![String::from("text_en")])
                .build(),
        ),
        (
            "display_name".to_string(),
            ArangoSearchViewLink::builder()
                .analyzers(vec![String::from("text_en")])
                .build(),
        ),
    ]);

    let links = ArangoSearchViewLink::builder()
        .include_all_fields(true)
        .fields(fields)
        .build();
    let properties = ArangoSearchViewPropertiesOptions::builder()
        .links(HashMap::from([("Identities".to_string(), links)]))
        .build();
    let view_options = ViewOptions::builder()
        .name(view_name.to_string())
        .typ(ViewType::ArangoSearchView)
        .properties(properties)
        .build();
    db.create_view(view_options).await?;
    Ok(db)
}
