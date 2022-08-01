use crate::{
    error::Error,
    graph::{
        edge::Hold,
        new_db_connection,
        vertex::contract::Chain,
        vertex::Identity,
        vertex::{contract::ContractCategory, Contract},
    },
    upstream::{knn3::Knn3, Fetcher, Platform, Target},
};

#[tokio::test]
async fn test_knn3() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
            .to_string()
            .to_lowercase(),
    );
    Knn3::fetch(&target).await?;

    let db = new_db_connection().await?;

    Identity::find_by_platform_identity(&db, &Platform::Ethereum, &target.identity()?)
        .await?
        .expect("Record not found");

    Contract::find_by_chain_address(
        &db,
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .unwrap();

    let _ = Hold::find_by_id_chain_address(
        &db,
        "vitalik.eth",
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .expect("Record not found");
    Ok(())
}

#[tokio::test]
async fn test_knn3_fail_get_result() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96044".to_string(),
    );
    let res = Knn3::fetch(&target).await?;
    assert_eq!(res.len(), 0);
    Ok(())
}
