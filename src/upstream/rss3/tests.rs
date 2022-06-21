mod tests {
    use crate::{error::Error, upstream::rss3::Rss3, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke_nft_rss3() -> Result<(), Error> {
        let rs: Rss3 = Rss3 {
            account: "0x6875e13A6301040388F61f5DBa5045E1bE01c657".to_string(),
            network: "ethereum".to_string(),
            tags: "NFT".to_string(),
        };

        let result = rs.fetch(None).await?;

        // print!(result);
        assert_ne!(result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_smoke_token_rss3() -> Result<(), Error> {
        let rs: Rss3 = Rss3 {
            account: "0x6875e13A6301040388F61f5DBa5045E1bE01c657".to_string(),
            network: "ethereum".to_string(),
            tags: "Token".to_string(),
        };

        let result = rs.fetch(None).await?;

        //println!("{}", result);
        assert_ne!(result.len(), 0);

        Ok(())
    }
}
