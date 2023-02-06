use crate::{
    error::Error,
    graph::new_db_connection,
    graph::vertex::{contract::Chain, Contract, Identity},
    upstream::unstoppable::Unstoppable,
    upstream::Platform,
    upstream::{Fetcher, Target},
};

#[tokio::test]
async fn test_fetch_domains_by_account() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0xCbCca6e22d90b8d2B829852a8D551e8410f40956".to_lowercase(),
    );
    let target2 = Target::Identity(Platform::Unstoppable, String::from("0xzella.crypto"));
    let _ = Unstoppable::fetch(&target).await?;
    let db = new_db_connection().await?;
    let found =
        Identity::find_by_platform_identity(&db, &target2.platform()?, &target2.identity()?)
            .await?
            .expect("Record not found");
    print!("found: {:?}", found);
    Ok(())
}

#[tokio::test]
async fn test_fetch_account_by_domain() -> Result<(), Error> {
    let target = Target::Identity(Platform::Unstoppable, String::from("88888888.888"));
    let target2 = Target::Identity(
        Platform::Ethereum,
        "0x2da822e59c68f4fb90a5f8dec39410602f45f35f".to_lowercase(),
    );
    let _ = Unstoppable::fetch(&target).await?;
    let db = new_db_connection().await?;
    let found =
        Identity::find_by_platform_identity(&db, &target2.platform()?, &target2.identity()?)
            .await?
            .expect("Record not found");
    print!("found: {:?}", found);
    Ok(())
}
