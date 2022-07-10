mod tests {
    use crate::{
        error::Error,
        graph::{new_db_connection, vertex::Identity},
        upstream::sybil_list::{prefetch, SybilList},
        upstream::{Fetcher, Platform},
        util::naive_now,
    };

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        // FIXME: too slow
        prefetch().await?;

        let sy: SybilList = SybilList {
            platform: Platform::Ethereum,
            identity: "0x4306D8e8AC2a9C893Ac1cd137a0Cd6966Fa6B6Ff".into(),
        };
        let fetched = sy.fetch().await?;

        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(&db, &sy.platform, &sy.identity)
            .await?
            .expect("Record not found");
        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

        assert_eq!(
            (Platform::Twitter, "MonetSupply".into()),
            *fetched.first().unwrap()
        );

        Ok(())
    }
}
