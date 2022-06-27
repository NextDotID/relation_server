#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::fetcher::{Upstream, fetcher};

    #[tokio::test]
    async fn test_fetcher_result() -> Result<(), Error> {
        let result = fetcher("github".to_string(), "fengshanshan".to_string()).await?;
        assert_ne!(result.len(), 0);
        Ok(())
    }
}
