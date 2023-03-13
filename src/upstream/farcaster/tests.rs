#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::farcaster::{
        get_farcaster_profile_by_signer, get_farcaster_profile_by_username,
    };

    #[tokio::test]
    async fn test_get_farcaster_profile_by_username() -> Result<(), Error> {
        let username = "zella";
        let data = get_farcaster_profile_by_username(&username)
            .await?
            .expect("Record not found");
        println!("data: {:?}", data);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_farcaster_profile_by_signer() -> Result<(), Error> {
        let address = "0xb86ff7e3f4e6186dfd25cff40605441d0c0481c4";
        let data = get_farcaster_profile_by_signer(&address)
            .await?
            .expect("Record not found");
        println!("data: {:?}", data);
        Ok(())
    }
}
