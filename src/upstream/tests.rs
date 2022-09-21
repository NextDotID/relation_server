use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target, UP_NEXT};
use crate::util::{queue_push, queue_unwrap};
use env_logger::Env;
use log::info;

#[tokio::test]
async fn test_fetcher_result() -> Result<(), Error> {
    fetch_all(Target::Identity(Platform::Twitter, "suji_yan".into()));
    let queue: Vec<Target> = queue_unwrap(&UP_NEXT);
    assert_eq!(1, queue.len());

    Ok(())
}

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    queue_push(&UP_NEXT, Target::Identity(Platform::Twitter, "0xsannie".into()));
    let result = fetch_one().await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_arc_mutex() -> Result<(), Error> {
    let async_task = tokio::spawn(async {
        queue_push(
            &UP_NEXT,
            Target::Identity(Platform::Twitter, "test321".into()),
        );
    });
    queue_push(
        &UP_NEXT,
        Target::Identity(Platform::Twitter, "test123".into()),
    );
    async_task.await.unwrap();

    let queue: Vec<Target> = queue_unwrap(&UP_NEXT);
    assert_eq!(2, queue.len());

    Ok(())
}
