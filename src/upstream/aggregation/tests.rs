mod tests {
    use crate::{error::Error, upstream::aggregation::Aggregation, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke_aggregation() -> Result<(), Error> {
        let ag: Aggregation = Aggregation {
            platform: "twitter".to_string(),
            identity: "0000".to_string(),
        };
        ag.fetch().await?;

        Ok(())
    }
}
