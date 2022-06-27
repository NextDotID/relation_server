mod tests {
    use crate::{error::Error, upstream::aggregation::Aggregation, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke_aggregation() -> Result<(), Error> {
        let ag: Aggregation = Aggregation {
            platform: "twitter".to_string(),
            identity: "0000".to_string(),
        };
        let result = ag.fetch().await?;
        assert_ne!(result.len(), 0);
        let first = result.first().unwrap();
        assert!(
            &first.from.identity.contains("0000"),
            "Greeting did not contain name, value was `{}`",
            first.from.identity
        );

        Ok(())
    }
}
