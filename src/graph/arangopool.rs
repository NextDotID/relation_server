use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseAccess, DatabaseConnection, OperationOptions};
use deadpool::managed::{Manager, Object, Pool, PoolConfig, RecycleError, RecycleResult, Timeouts};
use std::fmt;
// use deadpool::Runtime;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};
use tracing::{debug, error, trace};

#[derive(Clone, Debug, Deserialize)]
pub struct ArangoConfig {
    /// ArangoDB Connection Params
    /// See [Arangors Connection](arangors_lite::Connection::establish).
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: Option<String>,
    pub schema_path: Option<String>,
}

pub struct ArangoConnectionManager {
    pub host: String,
    pub username: String,
    pub password: String,
    pub db: String,
    pub schema_path: String,
}

impl ArangoConnectionManager {
    /// Creates a new [`ArangoConnectionManager`] with the given params.
    pub fn new(host: &str, username: &str, password: &str, db: &str, schema_path: &str) -> Self {
        Self {
            host: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            db: db.to_string(),
            schema_path: schema_path.to_string(),
        }
    }

    /// Creates a new [`ArangoConnectionManager`] with the given params.
    pub fn from_config(config: ArangoConfig) -> Result<Self, Error> {
        Ok(Self {
            host: config
                .host
                .ok_or(Error::ArangoConfigError(ArangoConfigError::MissingUrl))?,
            username: config
                .username
                .ok_or(Error::ArangoConfigError(ArangoConfigError::MissingUsername))?,
            password: config
                .password
                .ok_or(Error::ArangoConfigError(ArangoConfigError::MissingPassword))?,
            db: config
                .db
                .ok_or(Error::ArangoConfigError(ArangoConfigError::MissingDB))?,
            schema_path: config.schema_path.ok_or(Error::ArangoConfigError(
                ArangoConfigError::MissingSchemaPath,
            ))?,
        })
    }
}

pub struct ArangoConnection {
    connection: Object<ArangoConnectionManager>,
}

impl ArangoConnection {
    #[must_use]
    pub fn take(this: Self) -> DatabaseConnection {
        Object::take(this.connection)
    }
}

pub type ConnectionPool = Pool<ArangoConnectionManager>;

#[async_trait::async_trait]
impl Manager for ArangoConnectionManager {
    type Type = DatabaseConnection;
    type Error = Error;
    /// Create a new instance of the connection
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        debug!("Create a new instance of the arangodb connection");
        let connection = DatabaseConnection::builder()
            .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
            .with_auth_mode(AuthMode::Basic)
            .with_operation_options(OperationOptions::default())
            .with_schema_path(&C.db.schema_path)
            .build()
            .await?;
        Ok(connection)
    }

    /// Try to recycle a connection
    async fn recycle(&self, conn: &mut DatabaseConnection) -> RecycleResult<Error> {
        let check = conn.check_database(&self.db).await;
        match check {
            Ok(_) => match conn.database().aql_str::<i8>(r"RETURN 1").await {
                Ok(result) => match result {
                    _ if result[0] == 1 => {
                        trace!("Reuse exist DB connection.");
                        Ok(()) // recycle
                    }
                    _ => {
                        error!("Can not to recycle connection: arangodb response invalid");
                        Err(RecycleError::Message(
                            "Can not to recycle connection: arangodb response invalid".to_string(),
                        ))
                    }
                },

                Err(err) => {
                    error!("Can not to recycle connection: arangodb ping unsuccessful)");
                    Err(RecycleError::Message(err.to_string()))
                }
            },
            Err(err) => {
                error!("Can not to recycle connection: arangodb unreachable");
                Err(RecycleError::Message(err.to_string()))
            }
        }
    }
}

/// Create connection pool for arangodb
pub async fn new_connection_pool() -> Result<ConnectionPool, Error> {
    let manager = ArangoConnectionManager {
        host: C.db.host.to_string(),
        username: C.db.username.to_string(),
        password: C.db.password.to_string(),
        db: C.db.db.to_string(),
        schema_path: C.db.schema_path.to_string(),
    };

    let pool_config = PoolConfig {
        max_size: 24,
        timeouts: Timeouts::default(),
    };

    let pool = Pool::builder(manager)
        .config(pool_config)
        // .runtime(runtime)
        .build()
        .map_err(|err| Error::PoolError(err.to_string()));

    match pool {
        Ok(p) => Ok(p),
        Err(_) => todo!(),
    }
}

impl From<Object<ArangoConnectionManager>> for ArangoConnection {
    fn from(connection: Object<ArangoConnectionManager>) -> Self {
        Self { connection }
    }
}

impl Deref for ArangoConnection {
    type Target = DatabaseConnection;

    fn deref(&self) -> &DatabaseConnection {
        &self.connection
    }
}

impl DerefMut for ArangoConnection {
    fn deref_mut(&mut self) -> &mut DatabaseConnection {
        &mut self.connection
    }
}

impl AsRef<DatabaseConnection> for ArangoConnection {
    fn as_ref(&self) -> &DatabaseConnection {
        &self.connection
    }
}

impl AsMut<DatabaseConnection> for ArangoConnection {
    fn as_mut(&mut self) -> &mut DatabaseConnection {
        &mut self.connection
    }
}

impl Default for ArangoConfig {
    fn default() -> Self {
        Self {
            host: None,
            username: None,
            password: None,
            db: None,
            schema_path: None,
        }
    }
}

#[derive(Debug)]
pub enum ArangoConfigError {
    /// The `host` is invalid
    InvalidHost(String, url::ParseError),
    /// The `host` is `None`
    MissingUrl,
    /// The `username` is `None`
    MissingUsername,
    /// The `password` is None
    MissingPassword,
    /// The `db` is None
    MissingDB,
    /// The `schema_path` is None
    MissingSchemaPath,
}

impl fmt::Display for ArangoConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHost(host, e) => write!(f, "InvalidHost: {} - Error: {}", host, e),
            Self::MissingUrl => write!(f, "Missing URL"),
            Self::MissingUsername => write!(f, "Missing username"),
            Self::MissingPassword => write!(f, "Missing password"),
            Self::MissingDB => write!(f, "Missing db"),
            Self::MissingSchemaPath => write!(f, "Missing schema_path"),
        }
    }
}

impl std::error::Error for ArangoConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidHost(_, e) => Some(e),
            _ => None,
        }
    }
}
