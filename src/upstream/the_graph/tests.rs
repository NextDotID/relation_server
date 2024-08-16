use crate::{
    error::Error,
    tigergraph::{
        batch_upsert, batch_upsert_domains,
        edge::Hold,
        vertex::{Contract, Identity},
    },
    upstream::{
        the_graph::TheGraph, Chain, ContractCategory, DataFetcher, DataSource, DomainSearch,
        Fetcher, Platform, Target,
    },
    util::{make_http_client, parse_timestamp},
};
use tracing::{span, Instrument, Level};

#[tokio::test]
async fn test_find_ens_by_wallet() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x934b510d4c9103e6a87aef13b816fb080286d649".into(),
    );
    // let target = Target::NFT(
    //     Chain::Ethereum,
    //     ContractCategory::ENS,
    //     ContractCategory::ENS.default_contract_address().unwrap(),
    //     "sujiyan.eth".into(),
    // );
    let (_, all_edges) = TheGraph::batch_fetch(&target).await?;
    println!("all_edges {:?}", all_edges);

    let gsql_cli = make_http_client();
    if !all_edges.is_empty() {
        batch_upsert(&gsql_cli, all_edges).await?;
    }
    Ok(())
}

#[tokio::test]
async fn test_domain_search() -> Result<(), Error> {
    let name = "sujiyan";
    let all_edges = TheGraph::domain_search(name).await?;
    println!("data: {:?}", all_edges);
    let gsql_cli = make_http_client();
    if !all_edges.is_empty() {
        batch_upsert_domains(&gsql_cli, all_edges).await?;
    }
    Ok(())
}

#[tokio::test]
async fn test_find_wallet_by_ens() -> Result<(), Error> {
    let client = make_http_client();

    let target = Target::NFT(
        Chain::Ethereum,
        ContractCategory::ENS,
        ContractCategory::ENS.default_contract_address().unwrap(),
        "vitalik.eth".into(),
    );
    let address_targets = TheGraph::fetch(&target).await?;
    println!("targets {:?}", address_targets);
    assert!(!address_targets.is_empty());
    assert_eq!(
        address_targets.first().unwrap().identity().unwrap(),
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string()
    );

    Identity::find_by_platform_identity(
        &client,
        &Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045",
    )
    .await
    .expect("Fail to find identity")
    .expect("Record not found");

    Contract::find_by_chain_address(
        &client,
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await
    .expect("Fail to find ENS Contract")
    .expect("ENS Contract not found in DB");

    let hold = Hold::find_by_id_chain_address(
        &client,
        "vitalik.eth",
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .expect("Record not found");

    assert_eq!(hold.source, DataSource::TheGraph);
    assert_eq!(hold.fetcher, DataFetcher::RelationService);
    assert_eq!(hold.created_at, parse_timestamp("1497775154").ok());

    Ok(())
}

#[tokio::test]
async fn test_wrapped_ens_find_by_wallet() -> Result<(), Error> {
    // It has `nykma.eth` wrapped.
    let owner = "0x0da0ee86269797618032e56a69b1aad095c581fc".to_string();
    let target = Target::Identity(Platform::Ethereum, owner);

    let log = span!(Level::TRACE, "test_wrapped_domains");
    let address_targets = TheGraph::fetch(&target).instrument(log).await?;
    let _wrapped_ens = address_targets
        .iter()
        .find(|t| t.nft_id().unwrap() == "nykma.eth")
        .unwrap();

    Ok(())
}

#[tokio::test]
async fn test_wrapped_ens_find_by_ens() -> Result<(), Error> {
    let owner = "0x0da0ee86269797618032e56a69b1aad095c581fc";
    let ens = Target::NFT(
        Chain::Ethereum,
        ContractCategory::ENS,
        ContractCategory::ENS.default_contract_address().unwrap(),
        "nykma.eth".into(),
    );
    let log = span!(Level::TRACE, "test_wrapped_domains");
    let address_targets = TheGraph::fetch(&ens).instrument(log).await?;
    let _wrapped_ens = address_targets
        .iter()
        .find(|t| t.identity().unwrap() == owner)
        .unwrap();

    Ok(())
}
