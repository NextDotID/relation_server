use super::*;

#[tokio::test]
async fn test_fetch_success() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".into(),
    );
    let cli = make_http_client();
    ENSReverseLookup::fetch(&target).await?;
    let found = Identity::find_by_platform_identity(
        &cli,
        &target.platform().unwrap(),
        &target.identity().unwrap(),
    )
    .await
    .expect("Should find without error")
    .expect("Should find exact 1 result");
    assert_eq!(found.display_name, Some("vitalik.eth".to_string()));

    Ok(())
}
