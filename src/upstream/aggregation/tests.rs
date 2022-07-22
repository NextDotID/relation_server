mod tests {
    use crate::{
        error::Error,
        graph::new_db_connection,
        graph::vertex::{
            contract::{Chain, ContractCategory},
            Contract, Identity,
        },
        upstream::{aggregation::Aggregation, Target},
        upstream::{Fetcher, Platform},
    };
    use std::str::FromStr;

    #[tokio::test]
    async fn test_smoke_aggregation() -> Result<(), Error> {
        let target = Target::Identity(Platform::Twitter, "blake".to_string());
        let _ = Aggregation::fetch(&target).await?;

        let db = new_db_connection().await?;

        let _ = Identity::find_by_platform_identity(&db, &Platform::Twitter, "blakejamieson")
            .await?
            .expect("Record not found");

        let _ = Contract::find_by_chain_address(
            &db,
            &Chain::Ethereum,
            &ContractCategory::ENS.default_contract_address().unwrap(),
        )
        .await?
        .expect("Record not found");

        Ok(())
    }
}
