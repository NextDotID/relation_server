use crate::{
    error::{Error, Result},
    graph::{
        edge::{Edge, Hold, HoldRecord},
        vertex::{
            contract::{Chain, ContractCategory},
            Contract, Identity, IdentityRecord,
        },
    },
    upstream::{fetch_all, DataSource, Target, DataFetcher},
};
use aragog::{DatabaseConnection, DatabaseRecord};
use async_graphql::{Context, Object};
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

    /// NFT Category. See `availableNftCategories` for all values available.
    async fn category(&self, ctx: &Context<'_>) -> Result<ContractCategory> {
        // FIXME: Very stupid N+1 here.
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let to_record: DatabaseRecord<Contract> = self.record.to_record(db).await?;
        Ok(to_record.record.category)
    }

    /// On which chain?
    ///
    /// See `availableChains` for all chains supported by RelationService.
    async fn chain(&self, ctx: &Context<'_>) -> Result<Chain> {
        // FIXME: Very stupid N+1 here.
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let to_record: DatabaseRecord<Contract> = self.record.to_record(db).await?;
        Ok(to_record.record.chain)
    }

    /// Contract address of this Contract. Usually `0xHEX_STRING`.
    async fn address(&self, ctx: &Context<'_>) -> Result<String> {
        // FIXME: Very stupid N+1 here.
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let to_record: DatabaseRecord<Contract> = self.record.to_record(db).await?;
        Ok(to_record.record.address)
    }

    /// Token symbol (if any).
    async fn symbol(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        // FIXME: Very stupid N+1 here.
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let to_record: DatabaseRecord<Contract> = self.record.to_record(db).await?;
        Ok(to_record.record.symbol)
    }

    /// Which `Identity` does this NFT belong to.
    async fn owner(&self, ctx: &Context<'_>) -> Result<IdentityRecord> {
        // FIXME: Very stupid N+1 here.
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let identity: DatabaseRecord<Identity> = self.record.from_record(db).await?;
        Ok(identity.into())
    }

    /// Who collects this data.
    /// It works as a "data cleansing" or "proxy" between `source`s and us.
    async fn fetcher(&self) -> DataFetcher {
        self.fetcher
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

    /// List of all Contract Categoris supported by RelationService.
    async fn available_nft_categoris(&self) -> Vec<String> {
        ContractCategory::iter().map(|c| c.to_string()).collect()
    }

    /// Search an NFT.
    async fn nft(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "On which chain this NFT is. See `availableChains` for all values supported by RelationService."
        )]
        chain: Chain,
        #[graphql(
            desc = "What kind of this NFT is. See `availableNftCategoris` for all categories supported by RelationService."
        )]
        category: ContractCategory,
        #[graphql(
            desc = "ID of this NFT. For ENS, this is the name of the token (abc.eth). For other NFT, this is the NFT_ID in contract."
        )]
        id: String,
        #[graphql(
            desc = "Contract address of this NFT. Usually `0xHEX_STRING`. For `category: \"ENS\"`, this can be omitted."
        )]
        address: Option<String>,
    ) -> Result<Option<HoldRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let contract_address = address
            .or(category.default_contract_address())
            .ok_or(Error::GraphQLError("Contract address is required.".into()))?;
        let target = Target::NFT(
            chain,
            category,
            contract_address.clone(),
            id.clone(),
        );
        match Hold::find_by_id_chain_address(db, &id, &chain, &contract_address).await? {
            Some(hold) => {
                if hold.is_outdated() {
                    tokio::spawn(async move { fetch_all(target).await });
                }
                Ok(Some(hold))
            }

            None => {
                fetch_all(target).await?;
                Hold::find_by_id_chain_address(db, &id, &chain, &contract_address).await
            }
        }
    }
}
