#[cfg(test)]
mod tests {
    use crate::{graph::connect, error::Error};

    #[tokio::test]
    async fn test_connect() -> Result<(), Error> {
        connect().await?;
        Ok(())
    }
}
