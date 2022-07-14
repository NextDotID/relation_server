mod tests {
    use crate::{
        error::Error,
        upstream::{aggregation::Aggregation, Target},
        upstream::{Fetcher, Platform},
    };

    #[tokio::test]
    async fn test_smoke_aggregation() -> Result<(), Error> {
        let target = Target::Identity(Platform::Twitter, "0000".to_string());
        let res = Aggregation::fetch(&target).await?;
        assert_ne!(res.len(), 0);
        //println!("{:?}", res);

        Ok(())
    }
}
