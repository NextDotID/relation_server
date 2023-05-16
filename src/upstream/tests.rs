use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target};

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    let result = fetch_one(&Target::Identity(Platform::Twitter, "yeiwb".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_fetch_all() -> Result<(), Error> {
    fetch_all(vec![Target::Identity(Platform::Twitter, "yeiwb".into())], Some(1)).await?;

    Ok(())
}
