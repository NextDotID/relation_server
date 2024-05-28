#[cfg(test)]
mod tests {
    use crate::{
        error::Error,
        tigergraph::vertex::Identity,
        upstream::lensv2::LensV2,
        upstream::Platform,
        upstream::{Fetcher, Target},
        util::make_http_client,
    };

    #[tokio::test]
    async fn test_fetch_by_wallet() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            String::from("0x934B510D4C9103E6a87AEf13b816fb080286D649"),
        );
        let _ = LensV2::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(&client, &Platform::Lens, "sujiyan.lens")
            .await?
            .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_by_lens_handle() -> Result<(), Error> {
        let target = Target::Identity(Platform::Lens, String::from("xnownx.lens"));
        let _ = LensV2::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(
            &client,
            &Platform::Ethereum,
            &String::from("0x88a4FebB4572CF01967e5Ff9B6109dEA57168c6d").to_lowercase(),
        )
        .await?
        .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }
}
