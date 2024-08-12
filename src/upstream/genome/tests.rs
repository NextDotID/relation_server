#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::genome::{get_address, get_name, Genome};
    use crate::upstream::{DomainSearch, Fetcher, Platform, Target};

    #[tokio::test]
    async fn test_get_address() -> Result<(), Error> {
        let domain = "shiva";
        let domains = get_address(&domain).await?;
        println!("domains: {:?}", domains);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_name() -> Result<(), Error> {
        // let address = "0x782d7ff7214d3d9cb7a9afaf3f45a8f80cb73482";
        // let address = "0x99c19ab10b9ec8ac6fcda9586e81f6b73a298870";
        let address = "0x6d910bea79aaf318e7170c6fb8318d9c466b2164";
        let name = get_name(&address).await?;
        println!("name: {:?}", name);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0x6d910bea79aaf318e7170c6fb8318d9c466b2164".to_lowercase(),
        );
        // let target = Target::Identity(Platform::Genome, "shiva.gno".to_string());
        let _ = Genome::fetch(&target).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_domain_search() -> Result<(), Error> {
        let name = "vitalik";
        let edges = Genome::domain_search(name).await?;
        println!("data: {:?}", edges);
        Ok(())
    }
}
