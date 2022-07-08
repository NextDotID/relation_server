mod tests {
    use crate::{
        error::Error, 
        graph::{new_db_connection, vertex::Identity, vertex::NFT, vertex::nft::Chain}, 
        upstream::knn3::{Knn3, ENSCONTRACTADDR}, 
        upstream::{Fetcher, Platform}
    };

    #[tokio::test]
    async fn test_knn3() -> Result<(), Error> {
        let kn: Knn3 = Knn3 {
            platform:"ethereum".to_string(),
            identity: "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string(),
        };
        kn.fetch().await?;
       
        let db = new_db_connection().await?;
        let owner = Identity::find_by_platform_identity(&db, &Platform::Ethereum, &kn.identity)
        .await?
        .expect("Record not found");

        let ens = NFT::find_by_chain_contract_id(&db, &Chain::Ethereum, &ENSCONTRACTADDR.clone().to_string(), &"vitalik.eth".to_string()).await?.unwrap();

        let res = ens.belongs_to(&db).await.unwrap();
     
        assert_eq!(owner.uuid, res.unwrap().uuid);
        Ok(())
    }
}
