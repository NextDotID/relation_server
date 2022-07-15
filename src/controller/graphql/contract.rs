// use crate::{
//     error::{Error, Result},
//     graph::vertex::{
//         contract::{Chain, ContractCategory},
//         Contract, ContractRecord, IdentityRecord,
//     },
//     upstream::{fetch_all, Target},
// };
// use aragog::DatabaseConnection;
// use async_graphql::{Context, Object};
// use log::info;
// use strum::IntoEnumIterator;

// #[Object]
// impl ContractRecord {
//     /// UUID of this record. Managed by us RelationService.
//     async fn uuid(&self) -> String {
//         self.uuid.to_string()
//     }

//     /// Which Contract is this?
//     async fn category(&self) -> String {
//         self.category.to_string()
//     }

//     /// Contract address of this Contract. Usually `0xHEX_STRING`.
//     async fn address(&self) -> String {
//         self.address.clone()
//     }

//     /// On which chain?
//     ///
//     /// See `available_chains` for all chains supported by RelationService.
//     async fn chain(&self) -> String {
//         self.chain.to_string()
//     }

//     /// Token symbol (if any).
//     async fn symbol(&self) -> Option<String> {
//         self.symbol.clone()
//     }

//     /// When this data is updated (re-fetched) by RelationService.
//     async fn updated_at(&self) -> i64 {
//         self.updated_at.timestamp()
//     }

//     /// Which `Identity` does this Contract belong to.
//     async fn owner(&self, ctx: &Context<'_>) -> Result<Option<IdentityRecord>> {
//         let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
//         self.belongs_to(db).await
//     }

//     /// Which `Contract`s does this owner has?
//     async fn neighbor(&self, ctx: &Context<'_>) -> Result<Vec<ContractRecord>> {
//         let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
//         self.neighbors(db).await
//     }
// }

// #[derive(Default)]
// pub struct ContractQuery;

// #[Object]
// impl ContractQuery {
//     /// List of all chains supported by RelationService.
//     async fn available_chains(&self) -> Vec<String> {
//         Chain::iter().map(|c| c.to_string()).collect()
//     }

//     /// List of all Contract Categoris supported by RelationService.
//     async fn available_nft_categoris(&self) -> Vec<String> {
//         ContractCategory::iter().map(|c| c.to_string()).collect()
//     }

//     /// Search Contract
//     async fn nft(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(
//             desc = "Category of this Contract. See `available_nft_categoris` for all categories supported by RelationService."
//         )]
//         category: ContractCategory,
//         // contract: Option<String>,
//         #[graphql(
//             desc = "ID of this Contract. For ENS, this is the name of the token (abc.eth). For other Contract, this is the token ID in contract."
//         )]
//         chain: Chain,
//         #[graphql(
//             desc = "Contract address of this Contract. Usually `0xHEX_STRING`. For ENS, this can be omitted."
//         )]
//         contract: Option<String>,
//     ) -> Result<Option<ContractRecord>> {
//         let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
//         let contract_address =
//             contract
//                 .or(category.default_contract_address())
//                 .ok_or(Error::GraphQLError(
//                     "Contract address is required.".to_string(),
//                 ))?;
//         let target = Target::NFT(chain.clone(), category, id.clone());
//         match NFT::find_by_chain_contract(db, &chain, &contract_address).await? {
//             Some(nft) => {
//                 if nft.is_outdated() {
//                     info!(
//                         "NFT: {}/{}/{} is outdated. Refetching...",
//                         nft.record.chain, nft.record.category, nft.record.id
//                     );
//                     tokio::spawn(async move { fetch_all(target).await });
//                 }
//                 Ok(Some(nft))
//             }
//             None => {
//                 fetch_all(target).await?;
//                 NFT::find_by_chain_contract(db, &chain, &contract_address).await
//             }
//         }
//     }
// }
