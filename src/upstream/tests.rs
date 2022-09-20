use std::ops::DerefMut;

use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target};

use super::QUEUE;

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

#[tokio::test]
async fn test_arc_mutex() -> Result<(), Error> {
    let async_task = tokio::spawn(async move {
        QUEUE
            .clone()
            .lock()
            .unwrap()
            .deref_mut()
            .push(Target::Identity(Platform::Twitter, "test123".into()));
    });
    QUEUE
        .clone()
        .lock()
        .unwrap()
        .deref_mut()
        .push(Target::Identity(Platform::Twitter, "test321".into()));
    async_task.await.unwrap();

    let mutex_queue = QUEUE.clone();
    let result = mutex_queue.lock().unwrap();
    assert_eq!(2, result.len());

    Ok(())
}
