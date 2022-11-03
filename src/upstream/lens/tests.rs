use crate::{
    error::Error,
    graph::{
        edge::Hold,
        new_db_connection,
        vertex::contract::Chain,
        vertex::Identity,
        vertex::{contract::ContractCategory, Contract},
    },
    upstream::{lens::Lens, DataFetcher, DataSource, Fetcher, Platform, Target},
    util::parse_timestamp,
};

#[tokio::test]
async fn test_fetch_by_lens_profile() -> Result<(), Error> {
    let db = new_db_connection().await?;
    db.truncate().await;

    let target = Target::Identity(Platform::Lens, "stani.lens".into());
    let res = Lens::fetch(&target).await?;

    Ok(())
}
