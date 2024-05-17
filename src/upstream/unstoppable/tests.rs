#[cfg(test)]
mod tests {
    use crate::{
        error::Error,
        tigergraph::vertex::Identity,
        upstream::unstoppable::UnstoppableDomains,
        upstream::{unstoppable::fetch_domain, Platform},
        upstream::{Fetcher, Target},
        util::make_http_client,
    };

    #[tokio::test]
    async fn test_fetch_domains_by_account() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0xCbCca6e22d90b8d2B829852a8D551e8410f40956".to_lowercase(),
        );
        let _ = UnstoppableDomains::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(
            &client,
            &Platform::UnstoppableDomains,
            "0xzella.crypto",
        )
        .await?
        .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_domain() -> Result<(), Error> {
        let owners = "0xCbCca6e22d90b8d2B829852a8D551e8410f40956";
        let page = "";
        let data = fetch_domain(&owners, &page).await?;
        print!("data: {:?}", data);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_by_domain() -> Result<(), Error> {
        let target = Target::Identity(Platform::UnstoppableDomains, String::from("88888888.888"));
        let _ = UnstoppableDomains::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(
            &client,
            &Platform::Ethereum,
            "0x2da822e59c68f4fb90a5f8dec39410602f45f35f",
        )
        .await?
        .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }
}
