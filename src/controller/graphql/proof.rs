use async_graphql::{Context, Object};

use crate::error::Result;
use crate::graph::vertex::IdentityRecord;
use crate::graph::{edge::proof::ProofRecord, new_db_connection, vertex::Identity};

#[Object]
impl ProofRecord {
    async fn uuid(&self) -> String {
        self.uuid.to_string()
    }

    async fn source(&self) -> String {
        self.source.to_string()
    }

    async fn record_id(&self) -> Option<String> {
        self.record_id.clone()
    }

    async fn created_at(&self) -> Option<i64> {
        self.created_at.map(|ca| ca.timestamp())
    }

    async fn last_fetched_at(&self) -> i64 {
        self.last_fetched_at.timestamp()
    }

    async fn from(&self, _ctx: &Context<'_>) -> Result<IdentityRecord> {
        // TODO: connect me to context
        let db = new_db_connection().await?;

        let from_record: aragog::DatabaseRecord<Identity> = self.from_record(&db).await?;

        Ok(from_record.into())
    }
}
