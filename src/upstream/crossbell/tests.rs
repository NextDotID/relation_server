#[cfg(test)]
mod tests {
    use crate::{
        error::Error,
        tigergraph::vertex::Identity,
        upstream::crossbell::Crossbell,
        upstream::Platform,
        upstream::{DomainSearch, Fetcher, Target},
        util::make_http_client,
    };

    #[tokio::test]
    async fn test_fetch_crossbell_by_wallet() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0x0fefed77bb715e96f1c35c1a4e0d349563d6f6c0".to_lowercase(),
        );
        let _ = Crossbell::fetch(&target).await?;
        let client = make_http_client();
        let found =
            Identity::find_by_platform_identity(&client, &Platform::Crossbell, "joshua.csb")
                .await?
                .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_by_domain() -> Result<(), Error> {
        let target = Target::Identity(Platform::Crossbell, String::from("joshua.csb"));
        let _ = Crossbell::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(
            &client,
            &Platform::Ethereum,
            "0x0fefed77bb715e96f1c35c1a4e0d349563d6f6c0",
        )
        .await?
        .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_domain_search() -> Result<(), Error> {
        let name = "zzzzzzzzella";
        let edges = Crossbell::domain_search(name).await?;
        println!("data: {:?}", edges);
        Ok(())
    }
}
