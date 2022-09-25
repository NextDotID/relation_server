use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target};
use env_logger::Env;

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let result = fetch_one(&Target::Identity(Platform::Twitter, "yeiwb".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_fetch_all() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    fetch_all(Target::Identity(Platform::Twitter, "yeiwb".into())).await?;

    Ok(())
}
