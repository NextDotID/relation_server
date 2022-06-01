mod tests {
    use crate::{error::Error, upstream::sybil_list::SybilList, upstream::Fetcher};
    use crate::upstream::sybil_list::fetch2;

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let sy: SybilList = SybilList {};
        let result = sy.fetch(None).await?;
        println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
        Ok(())
    }


    #[tokio::test]
    async fn test_get_sybil_result_new() -> Result<(), Error> {
        let result = fetch2().await?;
        println!("{:?}", result.first());
        assert_ne!(result.len(), 0);
        Ok(())
    }
}
