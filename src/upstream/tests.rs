use crate::error::Error;
use crate::upstream::{
    batch_fetch_upstream, fetch_all, fetch_domains, fetch_one, Chain, ContractCategory, Platform,
    Target,
};

#[tokio::test]
async fn test_fetch_one_result() -> Result<(), Error> {
    let result = fetch_one(&Target::Identity(Platform::Twitter, "yeiwb".into())).await?;
    assert_ne!(result.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_batch_fetch_upstream() -> Result<(), Error> {
    let target = Target::Identity(Platform::Dotbit, "threebody.bit".into());

    let result = batch_fetch_upstream(&target).await?;
    println!("{:?}", result);
    Ok(())
}

#[tokio::test]
async fn test_fetch_all() -> Result<(), Error> {
    // fetch_all(
    //     vec![Target::Identity(
    //         Platform::Ethereum,
    //         "0xbe577c9e94d6a2598edde9089b78aef5a549cdb8".into(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::Identity(
    //         Platform::Ethereum,
    //         "0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85".into(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::Identity(
    //         Platform::Ethereum,
    //         "0x0da0ee86269797618032e56a69b1aad095c581fc".into(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    fetch_all(
        vec![Target::Identity(
            Platform::Ethereum,
            "0x934b510d4c9103e6a87aef13b816fb080286d649".into(),
        )],
        Some(5),
    )
    .await?;

    // fetch_all(
    //     vec![Target::Identity(
    //         Platform::Ethereum,
    //         "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".into(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::NFT(
    //         Chain::Ethereum,
    //         ContractCategory::ENS,
    //         ContractCategory::ENS.default_contract_address().unwrap(),
    //         "sujiyan.eth".to_string(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::NFT(
    //         Chain::Ethereum,
    //         ContractCategory::ENS,
    //         ContractCategory::ENS.default_contract_address().unwrap(),
    //         "vitalik.eth".to_string(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::Identity(Platform::CKB, "ckb1qzfhdsa4syv599s2s3nfrctwga70g0tu07n9gpnun9ydlngf5vsnwqggq7v6mzt3n8wv9y2n6h9z429ta0auek7v05yq0xdd39cenhxzj9fatj324z47h77vm0x869nu03m".into())],
    //     Some(5),
    // )
    // .await?;

    // fetch_all(
    //     vec![Target::Identity(
    //         Platform::Ethereum,
    //         "0x6d910bea79aaf318e7170c6fb8318d9c466b2164".into(),
    //     )],
    //     Some(5),
    // )
    // .await?;

    Ok(())
}

#[tokio::test]
async fn test_fetch_all_ens() -> Result<(), Error> {
    // аррӏе.eth
    fetch_all(
        vec![Target::NFT(
            Chain::Ethereum,
            ContractCategory::ENS,
            ContractCategory::ENS.default_contract_address().unwrap(),
            "brantly.eth".to_string(),
        )],
        Some(3),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_fetch_domains() -> Result<(), Error> {
    let name = "vitalik";
    fetch_domains(name).await?;
    Ok(())
}
