use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Chain, ContractCategory, Platform, Target};

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    let result = fetch_one(&Target::Identity(Platform::Twitter, "yeiwb".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_fetch_all() -> Result<(), Error> {
    fetch_all(
        vec![Target::Identity(Platform::Lens, "sujiyan.lens".into())],
        Some(3),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_fetch_all_ens() -> Result<(), Error> {
    fetch_all(
        vec![Target::NFT(
            Chain::Ethereum,
            ContractCategory::ENS,
            ContractCategory::ENS.default_contract_address().unwrap(),
            "yisiliu.eth".to_string(),
        )],
        Some(3),
    )
    .await?;

    Ok(())
}
