use tracing::Level;

use crate::error::Error;
use crate::upstream::{fetch_all, fetch_one, Platform, Target};

fn init_log() {
    let log_subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::DEBUG).finish();
    tracing::subscriber::set_global_default(log_subscriber).expect("Setting default subscriber failed");
}

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    init_log();
    let result = fetch_one(&Target::Identity(Platform::Twitter, "yeiwb".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_fetch_all() -> Result<(), Error> {
    init_log();
    fetch_all(Target::Identity(Platform::Twitter, "yeiwb".into())).await?;

    Ok(())
}
