use crate::{
    error::Error,
    upstream::rss3::Rss3,
    upstream::Platform,
    upstream::{Fetcher, Target},
};

#[tokio::test]
async fn test_fetch() -> Result<(), Error> {
    let target = Target::Identity(
        Platform::Ethereum,
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_lowercase(),
    );
    let (_targets, all_edges) = Rss3::batch_fetch(&target).await?;
    let json_raw_2 = serde_json::to_string(&all_edges).map_err(|err| Error::JSONParseError(err))?;
    println!("all_edges: {}", json_raw_2);
    println!("all_edges: {}", all_edges.len());

    Ok(())
}
