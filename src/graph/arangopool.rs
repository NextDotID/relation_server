use crate::{config::C, error::Error};
use aragog::{AuthMode, DatabaseAccess, DatabaseConnection, OperationOptions};
use deadpool::managed::{Manager, Object, Pool, PoolConfig, RecycleError, RecycleResult, Timeouts};
use std::fmt;
// use deadpool::Runtime;
use serde::Deserialize;
use std::ops::{Deref, DerefMut};
use tracing::{debug, error};

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

// pub struct ArangoConnectionPool {
//     // pub pool: Pool<ArangoConnection, Error>,
//     pub pool: Pool<ArangoConnectionManager>,
// }

// impl ArangoConnectionPool {
//     pub async fn connection(&self) -> Result<DatabaseConnection, Error> {
//         let pool_status = &self.pool.status();
//         debug!(
//             "==newtry Connection pool status: max_size={}, size={}, available={}",
//             pool_status.max_size, pool_status.size, pool_status.available
//         );
//         let conn = self
//             .pool
//             .get()
//             .await
//             .map_err(|err| Error::PoolError(err.to_string()))?;
//         Ok(Object::take(conn))
//     }
// }

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
