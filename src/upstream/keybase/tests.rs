mod tests {
    use crate::{
        error::Error,
        graph::new_db_connection,
        graph::vertex::Identity,
        upstream::keybase::Keybase,
        upstream::{Fetcher, Platform},
        util::naive_now,
    };

    #[tokio::test]
    async fn test_smoke_keybase() -> Result<(), Error> {
        let kb: Keybase = Keybase {
            platform: "github".to_string(),
            identity: "fengshanshan".to_string(),
        };
        kb.fetch().await?;
        let db = new_db_connection().await?;
        let found = Identity::find_by_platform_identity(
            &db,
            &Platform::Github,
            &"fengshanshan".to_string(),
        )
        .await?
        .expect("Record not found");

        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());

        Ok(())
    }
}
