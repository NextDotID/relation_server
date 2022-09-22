use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target, UP_NEXT};
use crate::util::{hashset_push, hashset_unwrap};
use env_logger::Env;

#[tokio::test]
async fn test_fetcher_result() -> Result<(), Error> {
    fetch_all(Target::Identity(Platform::Twitter, "suji_yan".into()));
    let set = hashset_unwrap(&UP_NEXT);
    assert_eq!(1, set.len());

    Ok(())
}

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    hashset_push(
        &UP_NEXT,
        Target::Identity(Platform::Twitter, "0xsannie".into()),
    );
    let result = fetch_one().await?;
    assert_ne!(result.len(), 0);

    Ok(())
}
