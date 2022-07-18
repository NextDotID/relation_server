mod tests {
    use crate::{
        error::Error,
        graph::new_db_connection,
        graph::{vertex::{Identity, Contract, contract::{Chain, ContractCategory}}},
        upstream::{aggregation::Aggregation, Target},
        upstream::{Fetcher, Platform},
    };

    #[tokio::test]
    async fn test_smoke_aggregation() -> Result<(), Error> {
        let target = Target::Identity(Platform::Twitter, "blake".to_string());
        let res = Aggregation::fetch(&target).await?;
        
        let db = new_db_connection().await?;

        let found =
        Identity::find_by_platform_identity(&db, &Platform::Twitter, "blakejamieson")
            .await?
            .expect("Record not found");

        let found_ens =
            Contract::find_by_chain_contract(&db, &Chain::Ethereum, &ContractCategory::ENS.default_contract_address().unwrap())
                .await?
                .expect("Record not found");

        Ok(())
    }
}
