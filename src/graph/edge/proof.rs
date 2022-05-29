use aragog::Record;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Edge to connect two `Identity`s.
#[derive(Debug, Clone, Serialize, Deserialize, Record)]
#[collection_name = "Proofs"]
pub struct Proof {
    pub uuid: Uuid,
    /// Upstream which provided this connection.
    /// TODO: enumerize this.
    pub upstream: String,
    /// ID of this connection in upstream platform.
    pub record_id: Option<String>,
    /// Connection creation time in upstream platform.
    pub created_at: Option<NaiveDateTime>,
    /// When this connection is fetched by RelationService.
    pub last_fetched_at: NaiveDateTime,
}
