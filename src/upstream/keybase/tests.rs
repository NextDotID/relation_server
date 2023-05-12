use crate::{
    error::Error,
    tigergraph::vertex::Identity,
    upstream::{keybase::Keybase, Target},
    upstream::{Fetcher, Platform},
    util::{make_http_client, naive_now},
};

#[tokio::test]
async fn test_smoke_keybase() -> Result<(), Error> {
    let target = Target::Identity(Platform::Github, "fengshanshan".into());
    Keybase::fetch(&target).await?;
    let cli = make_http_client();
    let found = Identity::find_by_platform_identity(&cli, &target.platform()?, &target.identity()?)
        .await?
        .expect("Record not found");

    assert!((found.updated_at.timestamp() - naive_now().timestamp()).abs() < 3);
    Ok(())
}
