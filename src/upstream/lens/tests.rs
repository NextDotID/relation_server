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
};

#[tokio::test]
async fn test_fetch_by_lens_profile() -> Result<(), Error> {
    let db = new_db_connection().await?;

    let target = Target::Identity(Platform::Lens, "stani.lens".into());
    let res = Lens::fetch(&target).await?;

    let owner = Identity::find_by_platform_identity(&db, &Platform::Lens, &target.identity()?)
        .await?
        .expect("Record not found");

    Ok(())
}

#[tokio::test]
async fn test_fetch_by_addrs() -> Result<(), Error> {
    let db = new_db_connection().await?;
    db.truncate().await;

    let target = Target::Identity(
        Platform::Ethereum,
        "0x7241dddec3a6af367882eaf9651b87e1c7549dff".to_string(),
    );
    let res = Lens::fetch(&target).await?;

    // let owner = Identity::find_by_platform_identity(&db, &Platform::Lens, &target.identity()?)
    //     .await?
    //     .expect("Record not found");

    Ok(())
}
