use crate::{
    error::Error,
    graph::edge::Hold,
    graph::new_db_connection,
    graph::vertex::{contract::Chain, Contract, Identity},
    upstream::rss3::Rss3,
    upstream::Platform,
    upstream::{Fetcher, Target},
};

#[tokio::test]
async fn test_smoke_nft_rss3() -> Result<(), Error> {
    // owns nft 0x60576a64851c5b42e8c57e3e4a5cf3cf4eeb2ed6 1484 on polygon
    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_lowercase(),
    );
    let _ = Rss3::fetch(&target).await?;

    let db = new_db_connection().await?;

    let owner = Identity::find_by_platform_identity(&db, &Platform::Ethereum, &target.identity()?)
        .await?
        .expect("Record not found");
    let contract = Contract::find_by_chain_address(
        &db,
        &Chain::Polygon,
        "0x60576a64851c5b42e8c57e3e4a5cf3cf4eeb2ed6",
    )
    .await?
    .unwrap();

    let _ = Hold::find_by_from_to_id(&db, &owner, &contract, "1484")
        .await
        .expect("Record not found");

    Ok(())
}
