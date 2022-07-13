mod tests {
    use crate::{
        error::Error,
        graph::new_db_connection,
        graph::{vertex::contract::Chain, vertex::Contract, vertex::Identity},
        upstream::rss3::Rss3,
        upstream::Fetcher,
        upstream::Platform,
    };

    #[tokio::test]
    async fn test_smoke_nft_rss3() -> Result<(), Error> {
        let rs: Rss3 = Rss3 {
            identity: "0x6875e13A6301040388F61f5DBa5045E1bE01c657"
                .to_lowercase()
                .to_string(),
            platform: "ethereum".to_string(),
        };
        rs.fetch().await?;
        let db = new_db_connection().await?;

        let owner = Identity::find_by_platform_identity(&db, &Platform::Ethereum, &rs.identity)
            .await?
            .expect("Record not found");
        let nft = Contract::find_by_chain_contract(
            &db,
            &Chain::Polygon,
            &"0x8f9772d0ed34bd0293098a439912f0f6d6e78e3f".to_string(),
        )
        .await?
        .unwrap();

        let res = nft.belongs_to(&db).await.unwrap();

        assert_eq!(owner.identity, res.unwrap().identity);
        Ok(())
    }
}
