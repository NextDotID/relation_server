use crate::upstream::Target;
use crate::{error::Error, upstream::proof_client::ProofClient, upstream::Fetcher};
use crate::{
    graph::new_db_connection, graph::vertex::Identity, upstream::Platform, util::naive_now,
};

#[tokio::test]
async fn test_smoke() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x2467ee73bb0c5acdeedf4e6cc5aa685741126872".into(),
    );
    ProofClient::fetch(&target).await?;

    let db = new_db_connection().await?;
    let found = Identity::find_by_platform_identity(&db, &target.platform()?, &target.identity()?)
        .await?
        .expect("Record not found");

    assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

    Ok(())
}
