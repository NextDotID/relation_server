use aragog::{DatabaseConnection, DatabaseRecord, Record};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Error,
    graph::vertex::{Identity, NFT},
};

use super::Edge;

#[derive(Clone, Deserialize, Serialize, Default, Record, Debug)]
#[collection_name = "Owns"]
pub struct Own {
    pub uuid: Uuid,
}

#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct OwnRecord(DatabaseRecord<Own>);

impl std::ops::Deref for OwnRecord {
    type Target = DatabaseRecord<Own>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OwnRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<DatabaseRecord<Own>> for OwnRecord {
    fn from(record: DatabaseRecord<Own>) -> Self {
        Self(record)
    }
}

#[async_trait::async_trait]
impl Edge<Identity, NFT, OwnRecord> for Own {
    /// Returns UUID of self.
    fn uuid(&self) -> Option<Uuid> {
        Some(self.uuid)
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        db: &DatabaseConnection,
        from: &DatabaseRecord<Identity>,
        to: &DatabaseRecord<NFT>,
    ) -> Result<OwnRecord, Error> {
        todo!()
    }

    /// Find an edge by UUID.
    async fn find_by_uuid(
        db: &DatabaseConnection,
        uuid: &Uuid,
    ) -> Result<Option<OwnRecord>, Error> {
        todo!()
    }
}
