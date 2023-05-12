use crate::upstream::Target;
use crate::{error::Error, upstream::proof_client::ProofClient, upstream::Fetcher};
use crate::{
    tigergraph::vertex::Identity,
    upstream::Platform,
    util::{make_http_client, naive_now},
};

#[tokio::test]
async fn test_smoke() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x2467ee73bb0c5acdeedf4e6cc5aa685741126872".into(),
    );
    ProofClient::fetch(&target).await?;

    let client = make_http_client();
    let found =
        Identity::find_by_platform_identity(&client, &target.platform()?, &target.identity()?)
            .await?
            .expect("Record not found");

    assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

    Ok(())
}

#[tokio::test]
async fn test_multiple_avatars() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x1cb1fa7d604e06cd8c596b5b7bcaaf5c5fdefd53".into(),
    );
    ProofClient::fetch(&target).await?;
    let client = make_http_client();
    let found = Identity::find_by_platform_identity(&client, &Platform::Twitter, "lyria_shan0127")
        .await?
        .expect("Record not found");
    assert_eq!(found.identity, "lyria_shan0127");

    Ok(())
}
