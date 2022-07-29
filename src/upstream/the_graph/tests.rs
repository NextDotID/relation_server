mod tests {
    use crate::{
        error::Error,
        graph::{
            edge::Hold,
            new_db_connection,
            vertex::contract::Chain,
            vertex::Identity,
            vertex::{contract::ContractCategory, Contract},
        },
        upstream::{the_graph::TheGraph, DataSource, Fetcher, Platform, Target},
    };

    #[tokio::test]
    async fn test_the_graph() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string(),
        );
        let targets = TheGraph::fetch(&target).await?;
        println!("targets {:?}", targets);

        let target_nft = Target::NFT(
            Chain::Ethereum,
            ContractCategory::ENS,
            ContractCategory::ENS.default_contract_address().unwrap(),
            "vitalik.eth".to_string(),
        );
        let address_targets = TheGraph::fetch(&target_nft).await?;
        println!("targets {:?}", address_targets);

        let db = new_db_connection().await?;
        let owner =
            Identity::find_by_platform_identity(&db, &Platform::Ethereum, &target.identity()?)
                .await?
                .expect("Record not found");

        let ens = Contract::find_by_chain_address(
            &db,
            &Chain::Ethereum,
            &ContractCategory::ENS.default_contract_address().unwrap(),
        )
        .await?
        .unwrap();

        let hold = Hold::find_by_id_chain_address(
            &db,
            "vitalik.eth",
            &Chain::Ethereum,
            &ContractCategory::ENS.default_contract_address().unwrap(),
        )
        .await?
        .expect("Record not found");

        assert_eq!(hold.source, DataSource::TheGraph);
        Ok(())
    }
}
