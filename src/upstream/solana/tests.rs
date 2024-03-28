#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::upstream::solana::{
        fetch_register_favourite, fetch_resolve_address, fetch_resolve_domains, fetch_reverse,
        get_handle_and_registry_key, get_rpc_client, get_twitter_registry, RPC_URL,
    };
    use rand::Rng;
    use sns_sdk::non_blocking::resolve::resolve_owner;

    pub fn generate_random_string(len: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..len)
            .map(|_| (rng.gen::<u8>() % 26) as char)
            .map(|c| (c as u8 + b'a') as char)
            .collect()
    }

    #[tokio::test]
    async fn test_fetch_resolve_domains() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        let res =
            fetch_resolve_domains(&client, "CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_resolve_address() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        let res = fetch_resolve_address(&client, "dtm").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_register_favourite() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));
        let res = fetch_register_favourite(&client, "HKKp49qGWXd639QsuH7JiLijfVW5UtCVY4s1n2HANwEA")
            .await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_reverse() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        let res = fetch_reverse(&client, "CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_twitter_registry() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        let res = get_twitter_registry(&client, "blueoceanshark").await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_handle_and_registry_key() -> Result<(), Error> {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        // CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9
        // 5k8SRiitUFPcUPLNB4eWwafXfYBP76iTx2P16xc99QYd
        let res =
            get_handle_and_registry_key(&client, "CLnUobvN8Fy7vhDMkQqNF7STxk5CT7MoePXvkgUGgdc9")
                .await?;
        println!("{:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn resolve() {
        let client = get_rpc_client(Some(RPC_URL.to_string()));

        // Domain does not exist
        let res = resolve_owner(&client, &generate_random_string(20))
            .await
            .unwrap();
        println!("res = {:?}", res);
        assert_eq!(res, None);
    }
}
