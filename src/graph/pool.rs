use crate::error::Error;
use arangors_lite::Connection;
use deadpool::managed::{Manager, RecycleError, RecycleResult};
use log::{debug, error};

pub struct ConnectionManager {
    pub host: String,
    pub username: String,
    pub password: String,
    pub db: String,
}

#[async_trait::async_trait]
impl Manager<Connection, Error> for ConnectionManager {
    /// Create a new instance of the arangors_lite::Connection
    async fn create(&self) -> Result<Connection, Error> {
        debug!("Create a new instance of the arangodb connection");
        let connection =
            Connection::establish_basic_auth(&self.host, &self.username, &self.password);
        Ok(connection.await?)
    }

    /// Try to recycle a connection
    async fn recycle(&self, conn: &mut Connection) -> RecycleResult<Error> {
        match conn.db(&self.db).await {
            Ok(db) => match db.aql_str::<i8>(r"RETURN 1").await {
                Ok(result) => match result {
                    _ if result[0] == 1 => {
                        debug!("Recycle exist connection");
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
                    error!("Can not to recycle connection: arangodb query unsuccessful)");
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
