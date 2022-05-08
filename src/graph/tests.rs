#[cfg(test)]
mod tests {
    use crate::{error::Error, graph::create_traversal};

    #[tokio::test]
    async fn test_connect() -> Result<(), Error> {
        create_traversal().await?;
        Ok(())
    }
}
