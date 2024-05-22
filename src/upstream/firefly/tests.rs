#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::firefly::{search_records, Firefly};
    use crate::upstream::{Fetcher, Platform, Target};

    #[tokio::test]
    async fn test_search_records() -> Result<(), Error> {
        let _identity = "kins";
        let _platform = Platform::Farcaster;

        let _identity_1 = "j0hnwang";
        let _platform_1 = Platform::Twitter;

        let _identity_2 = "0x88a4febb4572cf01967e5ff9b6109dea57168c6d";
        let _platform_2 = Platform::Ethereum;

        let records = search_records(&_platform_1, _identity_1).await?;
        println!("records: {:?}", records);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0x61ae970ac67ff4164ebf2fd6f38f630df522e5ef".to_lowercase(),
        );
        // let target = Target::Identity(Platform::Farcaster, "kins".to_string());
        let _ = Firefly::fetch(&target).await?;
        Ok(())
    }
}
