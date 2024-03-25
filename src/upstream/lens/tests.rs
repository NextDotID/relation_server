use crate::{
    error::Error,
    tigergraph::vertex::Identity,
    upstream::{lens::Lens, Fetcher, Platform, Target},
    util::make_http_client,
};

#[tokio::test]
async fn test_fetch_by_lens_profile() -> Result<(), Error> {
    let client = make_http_client();

    let target = Target::Identity(Platform::Lens, "stani.lens".into());
    Lens::fetch(&target).await?;

    Identity::find_by_platform_identity(&client, &target.platform()?, &target.identity()?)
        .await?
        .expect("Record not found");

    Ok(())
}

#[tokio::test]
async fn test_fetch_by_addrs() -> Result<(), Error> {
    let client = make_http_client();

    let target = Target::Identity(
        Platform::Ethereum,
        "0x7241dddec3a6af367882eaf9651b87e1c7549dff".to_string(),
    );
    Lens::fetch(&target).await?;

    Identity::find_by_platform_identity(&client, &target.platform()?, &target.identity()?)
        .await?
        .expect("Record not found");

    Ok(())
}
