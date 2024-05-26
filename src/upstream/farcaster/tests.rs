#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::farcaster::warpcast::{batch_fetch_by_signer, batch_fetch_by_username};
    use crate::upstream::types::Platform;

    #[tokio::test]
    async fn test_get_farcaster_profile_by_username() -> Result<(), Error> {
        let username = "suji";
        let data = batch_fetch_by_username(&Platform::Farcaster, &username).await?;
        println!("data: {:?}", data);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_farcaster_profile_by_signer() -> Result<(), Error> {
        let address = "0x934b510d4c9103e6a87aef13b816fb080286d649";
        let data = batch_fetch_by_signer(&Platform::Farcaster, &address).await?;
        println!("data: {:?}", data);
        Ok(())
    }
}
