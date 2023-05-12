use crate::{
    error::Error,
    tigergraph::edge::Hold,
    tigergraph::vertex::{Contract, Identity},
    upstream::rss3::Rss3,
    upstream::{Chain, Platform},
    upstream::{Fetcher, Target},
    util::make_http_client,
};

#[tokio::test]
async fn test_smoke_nft_rss3() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x934b510d4c9103e6a87aef13b816fb080286d649".to_lowercase(),
    );
    let _ = Rss3::fetch(&target).await?;
    let client = make_http_client();

    let owner =
        Identity::find_by_platform_identity(&client, &Platform::Ethereum, &target.identity()?)
            .await?
            .expect("Record not found");
    let contract = Contract::find_by_chain_address(
        &client,
        &Chain::Ethereum,
        "0x596cfe8d6709a86d51ff0c18ebf0e66561b08ae3",
    )
    .await?
    .unwrap();

    let _ = Hold::find_by_from_to_id(&client, &owner, &contract, "87")
        .await
        .expect("Record not found");

    Ok(())
}
