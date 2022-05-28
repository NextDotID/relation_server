use crate::upstream::Platform;

use aragog::Record;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, Record)]
#[collection_name = "Identities"]
pub struct Identity {
    pub uuid: Uuid,
    pub platform: Platform,
    pub identity: String,
    pub display_name: String,
    pub profile_url: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: NaiveDateTime,
    pub added_at: NaiveDateTime,
}

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}
