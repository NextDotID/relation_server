#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::space_id::{get_address, get_name, SpaceId};
    use crate::upstream::{Fetcher, Platform, Target};

    #[tokio::test]
    async fn test_get_address() -> Result<(), Error> {
        // let domain = "nopayable.bnb";
        let domain = "sujiyan.bnb";
        let address = get_address(&domain).await?;
        println!("address: {:?}", address.to_lowercase());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_name() -> Result<(), Error> {
        // 0xB86fF7E3F4E6186DfD25cFF40605441D0c0481c4
        let address = "0x934b510d4c9103e6a87aef13b816fb080286d649";
        let name = get_name(&address).await?;
        println!("name: {:?}", name);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Ethereum,
            "0x934b510d4c9103e6a87aef13b816fb080286d649".to_lowercase(),
        );
        let _ = SpaceId::fetch(&target).await?;
        Ok(())
    }
}
