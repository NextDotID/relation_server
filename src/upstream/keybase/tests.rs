mod tests {
    use crate::{error::Error, upstream::keybase::query, upstream::Fetcher};

    #[tokio::test]
    async fn test_smoke_keybase() -> Result<(), Error> {

        let result = query().await?;

        println!("{:?}", result);
     
        Ok(())
    }
}