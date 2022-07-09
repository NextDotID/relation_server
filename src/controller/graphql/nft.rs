use crate::{
    error::{Error, Result},
    graph::vertex::{
        nft::{Chain, NFTCategory},
        IdentityRecord, NFTRecord, NFT,
    },
};
use aragog::DatabaseConnection;
use async_graphql::{Context, Object};
use strum::IntoEnumIterator;

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

    /// Token ID in contract. Usually `uint256.to_string()`. For ENS, this will become `abc.eth`.
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

    /// List of all NFT Categoris supported by RelationService.
    async fn available_nft_categoris(&self) -> Vec<String> {
        NFTCategory::iter().map(|c| c.to_string()).collect()
    }

    /// Search NFT
    async fn nft(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "Category of this NFT. See `available_nft_categoris` for all categories supported by RelationService."
        )]
        category: NFTCategory,
        // contract: Option<String>,
        #[graphql(
            desc = "ID of this NFT. For ENS, this is the name of the token (abc.eth). For other NFT, this is the token ID in contract."
        )]
        id: String,
        #[graphql(
            desc = "On which chain does this NFT exist? See `available_chains` for all chains supported by RelationService."
        )]
        chain: Chain,
        #[graphql(
            desc = "Contract address of this NFT. Usually `0xHEX_STRING`. For ENS, this can be omitted."
        )]
        contract: Option<String>,
    ) -> Result<Option<NFTRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let contract_address =
            contract
                .or(category.default_contract_address())
                .ok_or(Error::GraphQLError(
                    "Contract address is required.".to_string(),
                ))?;
        match NFT::find_by_chain_contract_id(db, &chain, &contract_address, &id).await? {
            Some(nft) => {
                // FIXME: fetch NFT data here.
                // if nft.is_outdated() {
                // }
                Ok(Some(nft))
            }
            None => Ok(None), // FIXME: Really need to fetch NFT info here.
        }
    }
}
