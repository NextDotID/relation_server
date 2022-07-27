mod tests {
    use crate::{
        error::Error,
        graph::{new_db_connection, vertex::Identity},
        upstream::{
            sybil_list::{prefetch, SybilList},
            Target,
        },
        upstream::{Fetcher, Platform},
        util::naive_now,
    };

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        // FIXME: too slow
        prefetch().await?;

        let target = Target::Identity(
            Platform::Ethereum,
            "0x4306D8e8AC2a9C893Ac1cd137a0Cd6966Fa6B6Ff".into(),
        );
        let fetched = SybilList::fetch(&target).await?;

        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(
            &db,
            &target.platform()?,
            &target.identity()?.to_lowercase(),
        )
        .await?
        .expect("Record not found");
        assert_eq!(
            Target::Identity(Platform::Twitter, "MonetSupply".into()),
            *fetched.first().unwrap()
        );

        Ok(())
    }
}
