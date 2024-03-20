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
    async fn test_fetch_by_lens_handle() -> Result<(), Error> {
        let target = Target::Identity(Platform::Lens, String::from("sujiyan.lens"));
        let _ = LensV2::fetch(&target).await?;
        // let client = make_http_client();
        // let found = Identity::find_by_platform_identity(
        //     &client,
        //     &Platform::Ethereum,
        //     "0x0fefed77bb715e96f1c35c1a4e0d349563d6f6c0",
        // )
        // .await?
        // .expect("Record not found");
        // print!("found: {:?}", found);
        Ok(())
    }
}
