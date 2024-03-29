#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::tigergraph::vertex::Identity;
    use crate::upstream::solana::{
        fetch_register_favourite, fetch_resolve_address, fetch_resolve_domains,
        get_handle_and_registry_key, get_rpc_client, get_twitter_registry, Solana,
    };
    use crate::upstream::{Fetcher, Platform, Target};
    use crate::util::make_http_client;
    use rand::Rng;
    use sns_sdk::non_blocking::resolve::resolve_owner;

    // const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
    // const RPC_URL: &str = "https://api.testnet.solana.com";
    const RPC_URL: &str = "https://api.devnet.solana.com"; // sns-api.bonfida.com

    pub fn generate_random_string(len: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..len)
            .map(|_| (rng.gen::<u8>() % 26) as char)
            .map(|c| (c as u8 + b'a') as char)
            .collect()
    }

    #[tokio::test]
    async fn test_fetch_resolve_domains() -> Result<(), Error> {
        let client = get_rpc_client(RPC_URL.to_string());

        let res =
            fetch_resolve_domains(&client, "CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_resolve_address() -> Result<(), Error> {
        let client = get_rpc_client(RPC_URL.to_string());

        let res = fetch_resolve_address(&client, "dtm").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_register_favourite() -> Result<(), Error> {
        let client = get_rpc_client(RPC_URL.to_string());
        let res = fetch_register_favourite(&client, "5k8SRiitUFPcUPLNB4eWwafXfYBP76iTx2P16xc99QYd")
            .await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_twitter_registry() -> Result<(), Error> {
        let client = get_rpc_client(RPC_URL.to_string());

        let res = get_twitter_registry(&client, "blueoceanshark").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_handle_and_registry_key() -> Result<(), Error> {
        let client = get_rpc_client(RPC_URL.to_string());

        // CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9
        // 5k8SRiitUFPcUPLNB4eWwafXfYBP76iTx2P16xc99QYd
        let res =
            get_handle_and_registry_key(&client, "5k8SRiitUFPcUPLNB4eWwafXfYBP76iTx2P16xc99QYd")
                .await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn resolve() {
        let client = get_rpc_client(RPC_URL.to_string());

        // Domain does not exist
        let res = resolve_owner(&client, &generate_random_string(20))
            .await
            .unwrap();
        println!("res = {:?}", res);
        assert_eq!(res, None);
    }

    #[tokio::test]
    async fn test_fetch_solana() -> Result<(), Error> {
        let target = Target::Identity(Platform::SNS, String::from("bonfida.sol"));
        let _ = Solana::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(
            &client,
            &Platform::Solana,
            "HKKp49qGWXd639QsuH7JiLijfVW5UtCVY4s1n2HANwEA",
        )
        .await?
        .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_domains() -> Result<(), Error> {
        let target = Target::Identity(
            Platform::Solana,
            String::from("HKKp49qGWXd639QsuH7JiLijfVW5UtCVY4s1n2HANwEA"),
        );
        let _ = Solana::fetch(&target).await?;
        let client = make_http_client();
        let found = Identity::find_by_platform_identity(&client, &Platform::SNS, "bonfida.sol")
            .await?
            .expect("Record not found");
        print!("found: {:?}", found);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_twitter() -> Result<(), Error> {
        let target = Target::Identity(Platform::Twitter, String::from("dansform"));
        let _ = Solana::fetch(&target).await?;
        // let client = make_http_client();
        // let found = Identity::find_by_platform_identity(
        //     &client,
        //     &Platform::Solana,
        //     "HKKp49qGWXd639QsuH7JiLijfVW5UtCVY4s1n2HANwEA",
        // )
        // .await?
        // .expect("Record not found");
        // print!("found: {:?}", found);
        Ok(())
    }
}
