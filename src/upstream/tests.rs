use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target};

#[tokio::test]
async fn test_fetcher_result() -> Result<(), Error> {
    fetch_all(Target::Identity(Platform::Twitter, "suji_yan".into()))
        .await
        .expect("fetch_all should success");

    Ok(())
}

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    let result = fetch_one(&Target::Identity(Platform::Twitter, "0xsannie".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}
