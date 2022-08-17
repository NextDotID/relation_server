use crate::{
    error::Error,
    graph::{
        edge::Hold,
        new_db_connection,
        vertex::contract::Chain,
        vertex::Identity,
        vertex::{contract::ContractCategory, Contract},
    },
    upstream::{the_graph::TheGraph, DataFetcher, DataSource, Fetcher, Platform, Target},
    util::parse_timestamp,
};

#[tokio::test]
async fn test_find_ens_by_wallet() -> Result<(), Error> {
    let db = new_db_connection().await?;
    db.truncate().await;

    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".into(),
    );
    let targets = TheGraph::fetch(&target).await?;
    println!("targets {:?}", targets);

    Identity::find_by_platform_identity(&db, &Platform::Ethereum, &target.identity()?)
        .await
        .expect("Fail to find identity")
        .expect("Record not found");

    Contract::find_by_chain_address(
        &db,
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await
    .expect("Fail to find ENS Contract")
    .expect("ENS Contract not found in DB");

    let hold = Hold::find_by_id_chain_address(
        &db,
        "vitalik.eth",
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .expect("Record not found");

    assert_eq!(hold.source, DataSource::TheGraph);
    assert_eq!(hold.fetcher, DataFetcher::RelationService);
    assert_eq!(hold.created_at, parse_timestamp("1497775154").ok());
    Ok(())
}

#[tokio::test]
async fn test_find_wallet_by_ens() -> Result<(), Error> {
    let db = new_db_connection().await?;
    db.truncate().await;

    let target = Target::NFT(
        Chain::Ethereum,
        ContractCategory::ENS,
        ContractCategory::ENS.default_contract_address().unwrap(),
        "vitalik.eth".into(),
    );
    let address_targets = TheGraph::fetch(&target).await?;
    println!("targets {:?}", address_targets);
    assert!(!address_targets.is_empty());
    assert_eq!(
        address_targets.first().unwrap().identity().unwrap(),
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string()
    );

    Identity::find_by_platform_identity(
        &db,
        &Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045",
    )
    .await
    .expect("Fail to find identity")
    .expect("Record not found");

    Contract::find_by_chain_address(
        &db,
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await
    .expect("Fail to find ENS Contract")
    .expect("ENS Contract not found in DB");

    let hold = Hold::find_by_id_chain_address(
        &db,
        "vitalik.eth",
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .expect("Record not found");

    assert_eq!(hold.source, DataSource::TheGraph);
    assert_eq!(hold.fetcher, DataFetcher::RelationService);
    assert_eq!(hold.created_at, parse_timestamp("1497775154").ok());

    Ok(())
}
