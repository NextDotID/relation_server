#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::tigergraph::batch_upsert_domains;
    use crate::upstream::space_id::{get_address, get_name, v3::SpaceIdV3, SpaceId};
    use crate::upstream::{DomainSearch, Fetcher, Platform, Target};
    use crate::util::make_http_client;

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

    #[tokio::test]
    async fn test_domain_search() -> Result<(), Error> {
        let name = "sujiyan";
        let all_edges = SpaceIdV3::domain_search(name).await?;
        println!("data: {:?}", all_edges);

        let gsql_cli = make_http_client();
        if !all_edges.is_empty() {
            batch_upsert_domains(&gsql_cli, all_edges).await?;
        }
        Ok(())
    }
}
