use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseAccess, DatabaseConnection, OperationOptions};
use deadpool::managed::{Manager, Pool, RecycleError, RecycleResult};
use std::ops::Deref;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ArangoConnectionManager {
    pub host: String,
    pub username: String,
    pub password: String,
    pub db: String,
    pub schema_path: String,
}

// #[derive(Clone)]
pub struct ArcArangoConnection {
    pub connection: Arc<dyn DatabaseAccess + Send + Sync + 'static>,
}

impl ArcArangoConnection {
    pub fn connection(&self) -> &dyn DatabaseAccess {
        self.connection.deref()
    }
}

// #[derive(Clone)]

pub struct ConnectionPool {
    // pub pool: Pool<ArcArangoConnection, Error>,
    pub pool: Arc<Pool<ArcArangoConnection, Error>>,
}

impl ConnectionPool {
    pub async fn db(&self) -> Result<&dyn DatabaseAccess, Error> {
        let pool_status = &self.pool.status();
        debug!(
            "Connection pool status: max_size={}, size={}, available={}",
            pool_status.max_size, pool_status.size, pool_status.available
        );
        let connection = &self.pool.get().await;
        match connection {
            Ok(conn) => Ok(conn.connection()),
            Err(err) => Err(Error::PoolError(err.to_string())),
        }
    }
}

#[async_trait::async_trait]
impl Manager<ArcArangoConnection, Error> for ArangoConnectionManager {
    /// Create a new instance of the connection
    async fn create(&self) -> Result<ArcArangoConnection, Error> {
        debug!("Create a new instance of the arangodb connection");
        let connection = DatabaseConnection::builder()
            .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
            .with_auth_mode(AuthMode::Basic)
            .with_operation_options(OperationOptions::default())
            .with_schema_path(&C.db.schema_path)
            .build()
            .await?;
        let boxed_connection = ArcArangoConnection {
            connection: Arc::new(connection),
        };
        Ok(boxed_connection)
    }

    /// Try to recycle a connection
    async fn recycle(&self, conn: &mut ArcArangoConnection) -> RecycleResult<Error> {
        match conn
            .connection()
            .database()
            .aql_str::<i8>(r"RETURN 1")
            .await
        {
            Ok(result) => match result {
                _ if result[0] == 1 => {
                    debug!("==arc_conn Recycle exist connection");
                    Ok(()) // recycle
                }
                _ => {
                    error!("==arc_conn Can not to recycle connection: arangodb response invalid");
                    Err(RecycleError::Message(
                        "==arc_conn Can not to recycle connection: arangodb response invalid"
                            .to_string(),
                    ))
                }
            },
            Err(err) => {
                error!("==arc_conn Can not to recycle connection: arangodb unreachable");
                Err(RecycleError::Message(err.to_string()))
            }
        }
    }
}

/// Create connection pool for arangodb
pub async fn new_connection_pool() -> ConnectionPool {
    let max_pool_size = 24;
    let connection_pool = Pool::new(
        ArangoConnectionManager {
            host: C.db.host.to_string(),
            username: C.db.username.to_string(),
            password: C.db.password.to_string(),
            db: C.db.db.to_string(),
            schema_path: C.db.schema_path.to_string(),
        },
        max_pool_size,
    );
    debug!("Creating connection pool(db={})", &C.db.db);
    ConnectionPool {
        pool: Arc::new(connection_pool),
    }
}
