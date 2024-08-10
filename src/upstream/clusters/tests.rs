#[cfg(test)]
mod tests {
    use crate::{error::Error, upstream::clusters::Clusters, upstream::DomainSearch};

    #[tokio::test]
    async fn test_domain_search() -> Result<(), Error> {
        let name = "suji";
        let edges = Clusters::domain_search(name).await?;
        println!("data: {:?}", edges);
        Ok(())
    }
}
