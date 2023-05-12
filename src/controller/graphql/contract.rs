use crate::graph::vertex::contract::ContractRecord;
use crate::upstream::{Chain, ContractCategory};
use async_graphql::Object;
use uuid::Uuid;

#[Object]
impl ContractRecord {
    /// UUID of this record.
    async fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// What kind of Contract is it?
    async fn category(&self) -> ContractCategory {
        self.category
    }

    /// Domain Name system
    async fn address(&self) -> String {
        self.address.clone()
    }

    /// On which chain?
    async fn chain(&self) -> Chain {
        self.chain
    }

    /// Token symbol
    async fn symbol(&self) -> Option<String> {
        self.symbol.clone()
    }

    /// When this connection is fetched by us RelationService.
    async fn updated_at(&self) -> i64 {
        self.updated_at.timestamp()
    }
}
