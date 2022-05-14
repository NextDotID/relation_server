mod tests {
    use crate::{error::Error, upstream::proof_client::ProofClient, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke() -> Result<(), Error> {
        let pf: ProofClient = ProofClient {
            base: "http://localhost:9800".to_string(),
            persona: "0x03666b700aeb6a6429f13cbb263e1bc566cd975a118b61bc796204109c1b351d19".to_string()
        };
        let result = pf.fetch(Some(" ".to_string())).await?;

        println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
        Ok(())
    }
}
