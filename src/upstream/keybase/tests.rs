mod tests {
    use crate::{
        error::Error,
        graph::new_db_connection,
        graph::vertex::Identity,
        upstream::{keybase::Keybase, Target},
        upstream::{Fetcher, Platform},
        util::naive_now,
    };

    #[tokio::test]
    async fn test_smoke_keybase() -> Result<(), Error> {
        let target = Target::Identity(Platform::Github, "fengshanshan".into());
        Keybase::fetch(&target).await?;
        let db = new_db_connection().await?;
        let found =
            Identity::find_by_platform_identity(&db, &target.platform()?, &target.identity()?)
                .await?
                .expect("Record not found");

        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());
        Ok(())
    }
}
