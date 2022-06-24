mod tests {
    use crate::{error::Error, upstream::knn3::Knn3, upstream::Fetcher};

    #[tokio::test]
    async fn test_knn3() -> Result<(), Error> {
        let rs: Knn3 = Knn3 {
            account: "0x6875e13A6301040388F61f5DBa5045E1bE01c657".to_string(),
        };
        let result = rs.fetch(None).await?;

        // print!(result);
        assert_ne!(result.len(), 0);

        Ok(())
    }
}

