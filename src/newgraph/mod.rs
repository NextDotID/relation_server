mod test;
pub mod edge;
pub mod vertex;
use arangors_lite::{Connection, Database, ClientError};
use serde_json::value::Value;
use serde_json::from_value;
use crate::newgraph::vertex::identity::Path;

pub async fn new_raw_db_connection() -> Result<Database, ClientError> {
    let conn = Connection::establish_basic_auth(
        "http://localhost:8529", "root", "ieNgoo5roong9Chu")
        .await
        .unwrap();
    let db = conn.db("relation_server_development")
        .await?;
    Ok(db)
}