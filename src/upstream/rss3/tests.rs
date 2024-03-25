use std::collections::HashMap;

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
        "0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85",
    )
    .await?
    .unwrap();

    let filters = HashMap::from([("id".to_string(), "maskbook.eth".to_string())]);
    let record = Hold::find_by_from_to(&client, &owner, &contract, Some(filters))
        .await?
        .and_then(|r| r.first().cloned())
        .expect("Record not found");
    let json_raw = serde_json::to_string(&record).map_err(|err| Error::JSONParseError(err))?;
    println!("found: {}", json_raw);
    Ok(())
}
