mod tests {
    use crate::{error::Error, upstream::proof_client::ProofClient, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke() -> Result<(), Error> {
        let pf: ProofClient = ProofClient {
            persona: "0x02d7c5e01bedf1c993f40ec302d9bf162620daea93a7155cd9a8019ae3a2c2a476"
                .to_string(),
        };
        let result = pf.fetch(None).await?;

        //println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
        Ok(())
    }
}
