#[cfg(test)]
mod tests {
    use crate::{graph::connect, error::Error};

    #[test]
    fn test_connect() -> Result<(), Error> {
        connect()?;
        Ok(())
    }
}
