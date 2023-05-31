use crate::{
    error::Error,
    tigergraph::vertex::{Contract, Identity},
    upstream::{aggregation::Aggregation, Target},
    upstream::{Chain, ContractCategory, Fetcher, Platform},
    util::{make_http_client, timestamp_to_naive},
};

#[tokio::test]
async fn test_smoke_aggregation() -> Result<(), Error> {
    let target = Target::Identity(Platform::Twitter, "blake".to_string());
    let _ = Aggregation::fetch(&target).await?;

    let cli = make_http_client();

    let _ = Identity::find_by_platform_identity(&cli, &Platform::Twitter, "blakejamieson")
        .await?
        .expect("Record not found");

    let _ = Contract::find_by_chain_address(
        &cli,
        &Chain::Ethereum,
        &ContractCategory::ENS.default_contract_address().unwrap(),
    )
    .await?
    .expect("Record not found");

    Ok(())
}

#[tokio::test]
async fn test_parse_timestamp() -> Result<(), Error> {
    let ct_time = "1654669460431".to_string();
    println!("nt_time original{}", ct_time.parse::<i64>().unwrap() % 1000);
    let ns_time: u32 = (ct_time.parse::<i64>().unwrap() % 1000).try_into().unwrap();
    println!("nt_time {}", ns_time);

    let created_at = timestamp_to_naive(
        ct_time.parse::<i64>().unwrap() / 1000,
        ns_time,
    );
    let updated_at = timestamp_to_naive(ct_time.parse::<i64>().unwrap() / 1000, ns_time).unwrap();
    println!("{}", ct_time.parse::<i64>().unwrap());
    println!("{:?}", created_at);
    println!("{}", updated_at.timestamp());

    Ok(())
}
