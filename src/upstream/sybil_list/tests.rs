mod tests {
    use crate::{error::Error, upstream::sybil_list::SybilList, upstream::Fetcher};

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let sy: SybilList = SybilList {};
        let result = sy.fetch(None).await?;
        assert_ne!(result.len(), 0);
        Ok(())
    }


    #[tokio::test]
    async fn test_sybil_ability() -> Result<(), Error> {
        let result = SybilList::ability();
        println!("{:?}", result);
        assert_ne!(result.len(), 0);
        Ok(())
    }
}
