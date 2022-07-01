use aragog::DatabaseConnection;
use async_graphql::{Context, Object};
use strum::IntoEnumIterator;

use crate::error::{Error, Result};
use crate::graph::vertex::{nft::Chain, IdentityRecord, NFTRecord};

#[Object]
impl NFTRecord {
    /// UUID of this record. Managed by us RelationService.
    async fn uuid(&self) -> String {
        self.uuid.to_string()
    }

    /// Which NFT is this?
    async fn category(&self) -> String {
        self.category.to_string()
    }

    /// Contract address of this NFT. Usually `0xHEX_STRING`.
    async fn contract(&self) -> String {
        self.contract.clone()
    }

    /// Token ID in contract. Usually `uint256.to_string()`.
    async fn id(&self) -> String {
        self.id.clone()
    }

    /// On which chain?
    ///
    /// See `available_chains` for all chains supported by RelationService.
    async fn chain(&self) -> String {
        self.chain.to_string()
    }

    /// Token symbol (if any).
    async fn symbol(&self) -> Option<String> {
        self.symbol.clone()
    }

    /// When this data is updated (re-fetched) by RelationService.
    async fn updated_at(&self) -> i64 {
        self.updated_at.timestamp()
    }

    /// Which `Identity` does this NFT belong to.
    async fn owner(&self, ctx: &Context<'_>) -> Result<Option<IdentityRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        self.belongs_to(db).await
    }

    /// Which `NFT`s does this owner has?
    async fn neighbor(&self, ctx: &Context<'_>) -> Result<Vec<NFTRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        self.neighbors(db).await
    }
}

#[derive(Default)]
pub struct NFTQuery;

#[Object]
impl NFTQuery {
    /// List of all chains supported by RelationService.
    async fn available_chains(&self) -> Vec<String> {
        Chain::iter().map(|c| c.to_string()).collect()
    }
}
