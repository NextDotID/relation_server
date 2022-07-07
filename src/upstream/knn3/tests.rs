mod tests {
    use crate::{
        error::Error, 
        graph::{new_db_connection, vertex::Identity}, 
        upstream::knn3::Knn3, 
        upstream::{Fetcher, Platform}
    };

    #[tokio::test]
    async fn test_knn3() -> Result<(), Error> {
        let kn: Knn3 = Knn3 {
            platform:"ethereum".to_string(),
            identity: "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string(),
        };
        kn.fetch().await?;
        kn.fetch().await?;
        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(&db, &Platform::Ethereum, &kn.identity)
        .await?
        .expect("Record not found");
        let neighbors = found.neighbors(&db, 1, None).await?;
        assert_ne!(neighbors.len(), 0);
        Ok(())
    }
}
