mod tests {
    use crate::{
        error::Error,
        upstream::keybase::Keybase,
        upstream::{Fetcher, Platform},
    };

    #[tokio::test]
    async fn test_smoke_keybase() -> Result<(), Error> {
        let kb: Keybase = Keybase {
            platform: "github".to_string(),
            identity: "fengshanshan".to_string(),
        };

        let result = kb.fetch().await?;

        //println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
        let item = result
            .iter()
            .find(|c| &&c.to.identity == &&"fengshanshan".to_string())
            .unwrap();
        assert_eq!(item.to.platform, Platform::Github);

        Ok(())
    }
}
