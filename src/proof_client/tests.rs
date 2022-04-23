#[cfg(test)]
mod tests {
    use crate::{error::Error, proof_client::query};
    const PROOF_SERVICE_URL: &str = "https://proof-service.nextnext.id"; // Staging

    #[tokio::test]
    async fn test_smoke() -> Result<(), Error> {
        let result = query(
            PROOF_SERVICE_URL,
            "0x000000000000000000000000000000000000000000000000000000000000000000",
        )
        .await
        .unwrap();
        assert_eq!(result.ids.len(), 0);
        Ok(())
    }
}
