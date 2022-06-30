mod tests {
    use warp::addr;

    use crate::{error::Error, upstream::proof_client::ProofClient, upstream::Fetcher};
    use crate::{
        graph::new_db_connection, graph::vertex::Identity, upstream::Platform, util::naive_now,
    };

    #[tokio::test]
    async fn test_smoke() -> Result<(), Error> {
        let addr = String::from("0x2467Ee73Bb0c5AcDeEdf4E6cC5aA685741126872");
        let pf: ProofClient = ProofClient {
            platform: "ethereum".to_string(),
            identity: addr.clone(),
        };
        pf.fetch().await?;

        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(&db, &Platform::Ethereum, addr.as_str())
            .await?
            .expect("Record not found");

        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

        Ok(())
    }
}
