use crate::{
    graph::{edge::HoldRecord, vertex::contract::Chain},
    upstream::DataSource,
};
use async_graphql::Object;
use strum::IntoEnumIterator;
use uuid::Uuid;

#[Object]
impl HoldRecord {
    /// UUID of this record.
    async fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Data source (upstream) which provides this info.
    /// Theoretically, Contract info should only be fetched by chain's RPC server,
    /// but in practice, we still rely on third-party cache / snapshot service.
    async fn source(&self) -> DataSource {
        self.source
    }
    /// Transaction info of this connection.
    /// i.e. in which `tx` the Contract is transferred / minted.
    /// In most case, it is a `"0xVERY_LONG_HEXSTRING"`.
    /// It happens that this info is not provided by `source`, so we treat it as `Option<>`.
    async fn transaction(&self) -> Option<String> {
        self.transaction.clone()
    }
    /// NFT_ID in contract / ENS domain / anything can be used as an unique ID to specify the held object.
    /// It must be one here.
    /// Tips: NFT_ID of ENS is a hash of domain. So domain can be used as NFT_ID.
    async fn id(&self) -> String {
        self.id.clone()
    }
    /// When the transaction happened. May not be provided by upstream.
    async fn created_at(&self) -> Option<i64> {
        self.created_at.map(|dt| dt.timestamp())
    }
    /// When this HODL™ relation is fetched by us RelationService.
    async fn updated_at(&self) -> i64 {
        self.updated_at.timestamp()
    }
}

#[derive(Default)]
pub struct HoldQuery {}

#[Object]
impl HoldQuery {
    /// List of all chains supported by RelationService.
    async fn available_chains(&self) -> Vec<String> {
        Chain::iter().map(|c| c.to_string()).collect()
    }

    // TODO: move all `contract.rs` query to here.
}
