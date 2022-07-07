mod tests {
    use crate::{
        error::Error, 
        upstream::rss3::Rss3, 
        upstream::Fetcher,
        upstream::Platform,
        graph::new_db_connection,
        graph::vertex::Identity,
    };

    #[tokio::test]
    async fn test_smoke_nft_rss3() -> Result<(), Error> {
        let rs: Rss3 = Rss3 {
            identity: "0x6875e13A6301040388F61f5DBa5045E1bE01c657".to_string(),
            platform: "ethereum".to_string(),
        };
        rs.fetch().await?;
        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(&db, &Platform::Ethereum, &rs.identity)
        .await?
        .expect("Record not found");
        let neighbors = found.neighbors(&db, 1, None).await?;
        assert_ne!(neighbors.len(), 0);
        Ok(())
    }
}
