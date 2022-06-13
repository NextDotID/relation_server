mod tests {
    use crate::{error::Error, upstream::keybase::Keybase, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke_keybase() -> Result<(), Error> {

        let kb: Keybase = Keybase {
            platform: "github".to_string(),
            identity: "fengshanshan".to_string(),
        };

        let result = kb.fetch(None).await?;

        //println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
     
        Ok(())
    }
}
