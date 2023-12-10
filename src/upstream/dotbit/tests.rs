use crate::upstream::dotbit::{get_req_params, CoinType, ReverseRecordRequest};
use crate::upstream::Target;
use crate::{error::Error, upstream::dotbit::DotBit, upstream::Fetcher};
use crate::{
    tigergraph::vertex::Identity, upstream::Platform, util::make_http_client, util::naive_now,
};

#[tokio::test]
async fn test_smoke_dotbit_by_dotbit_identity() -> Result<(), Error> {
    let target = Target::Identity(Platform::Dotbit, "threebody.bit".into());

    DotBit::fetch(&target).await?;

    let client = make_http_client();
    let found =
        Identity::find_by_platform_identity(&client, &target.platform()?, &target.identity()?)
            .await?
            .expect("Record not found");
    tracing::debug!("found {:?}", found);
    // assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

    Ok(())
}

#[tokio::test]
async fn test_dotbit_account_list() -> Result<(), Error> {
    let request_params =
        get_req_params(&CoinType::ETH, "0x9176acd39a3a9ae99dcb3922757f8af4f94cdf3c");
    let params = ReverseRecordRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "das_accountList".to_string(),
        params: vec![request_params],
    };
    let json_raw = serde_json::to_string(&params).map_err(|err| Error::JSONParseError(err))?;
    // println!("{}", json_raw);
    tracing::info!("params {:?}", json_raw);
    Ok(())
}

#[tokio::test]
async fn test_dotbit_reverse_record() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0x9176acd39a3a9ae99dcb3922757f8af4f94cdf3c".into(),
    );

    DotBit::fetch(&target).await?;

    let client = make_http_client();
    let found =
        Identity::find_by_platform_identity(&client, &target.platform()?, &target.identity()?)
            .await?
            .expect("Record not found");
    tracing::debug!("found {:?}", found);
    assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

    Ok(())
}

#[tokio::test]
async fn test_smoke_dotbit_reverse_record() -> Result<(), Error> {
    //0x9176acd39a3a9ae99dcb3922757f8af4f94cdf3c holds justing.bit, resolve => "justing.bit"
    //0x4271B15dCa69f8C1c942c64028dBd3B84c5D03B0 holds test0920.bit, resolve => ""
    let target = Target::Identity(
        Platform::Ethereum,
        "0x4271B15dCa69f8C1c942c64028dBd3B84c5D03B0".into(),
    );
    assert_eq!(DotBit::fetch(&target).await.is_err(), true);

    let target2 = Target::Identity(
        Platform::Ethereum,
        "0X9176ACD39A3A9AE99DCB3922757F8AF4F94CDF3C".into(),
    );
    DotBit::fetch(&target2).await?;
    let client = make_http_client();

    assert_eq!(
        Identity::find_by_platform_identity(&client, &target2.platform()?, &target2.identity()?)
            .await?
            .is_none(),
        true
    );
    Identity::find_by_platform_identity(
        &client,
        &target2.platform()?,
        &target2.identity()?.to_ascii_lowercase(),
    )
    .await?
    .expect("Record not found");

    Identity::find_by_platform_identity(&client, &Platform::Dotbit, "justing.bit")
        .await?
        .expect("Record not found");

    Ok(())
}
