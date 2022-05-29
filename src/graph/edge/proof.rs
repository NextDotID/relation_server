use aragog::{
    query::{Comparison, QueryResult},
    DatabaseConnection, DatabaseRecord, EdgeRecord, Record,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::{vertex::Identity, Edge},
    upstream::DataSource,
};

/// Edge to connect two `Identity`s.
#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[collection_name = "Proofs"]
pub struct Proof {
    pub uuid: Uuid,
    /// Data source (upstream) which provided this connection.
    pub source: DataSource,
    /// ID of this connection in upstream platform to locate (if any).
    pub record_id: Option<String>,
    /// Connection creation time in upstream platform (if any).
    pub created_at: Option<NaiveDateTime>,
    /// When this connection is fetched by RelationService.
    pub last_fetched_at: NaiveDateTime,
}

#[async_trait::async_trait]
impl Edge<Identity, Identity> for Proof {
    fn uuid(&self) -> Option<Uuid> {
        Some(self.uuid)
    }

    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: Uuid,
    ) -> Result<Option<DatabaseRecord<EdgeRecord<Self>>>, Error> {
        let result: QueryResult<EdgeRecord<Proof>> = EdgeRecord::<Proof>::query()
            .filter(Comparison::field("uuid").equals_str(uuid).into())
            .call(db)
            .await?;

        if result.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(result.first().unwrap().to_owned()))
        }
    }

    /// TODO: Should impl dedup.
    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &Identity,
        to: &Identity,
    ) -> Result<Option<DatabaseRecord<EdgeRecord<Self>>>, Error> {
        todo!()
    }
}
