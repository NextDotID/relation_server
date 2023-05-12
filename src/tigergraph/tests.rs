#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::error::Error;
    use crate::tigergraph::{
        create_contract_to_identity_resolve_record, create_identity_domain_resolve_record,
        create_identity_to_contract_hold_record,
        create_identity_to_contract_reverse_resolve_record,
        create_identity_to_identity_hold_record, create_identity_to_identity_proof_two_way_binding,
    };
    use crate::{
        tigergraph::{
            edge::{Hold, Proof, Resolve},
            vertex::{Contract, Identity, NeighborsResponse},
        },
        upstream::{Chain, ContractCategory, DataSource, DomainNameSystem, Platform, ProofLevel},
        util::make_http_client,
    };

    #[tokio::test]
    async fn test_create_i2i_proof_two_way_binding() -> Result<(), Error> {
        let client = make_http_client();
        let mut from = Identity::default();
        let mut to = Identity::default();
        from.uuid = Some(Uuid::new_v4());
        to.uuid = Some(Uuid::new_v4());

        from.identity = "j".to_string();
        from.display_name = Some("jjjjjkkkk".to_string());
        to.identity = "k".to_string();

        from.platform = Platform::Ethereum;
        to.platform = Platform::NextID;

        // let json_raw = serde_json::to_string(&from).map_err(|err| Error::JSONParseError(err))?;
        // println!("{}", json_raw);

        let mut proof_forward = Proof::default();
        let mut proof_backward = Proof::default();
        proof_forward.source = DataSource::NextID;
        proof_backward.source = DataSource::NextID;
        proof_forward.uuid = Uuid::new_v4();
        proof_backward.uuid = Uuid::new_v4();
        proof_forward.level = ProofLevel::VeryConfident;
        proof_backward.level = ProofLevel::VeryConfident;
        create_identity_to_identity_proof_two_way_binding(
            &client,
            &from,
            &to,
            &proof_forward,
            &proof_backward,
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_create_i2i_hold_record() -> Result<(), Error> {
        let client = make_http_client();
        let mut from = Identity::default();
        let mut to = Identity::default();
        from.uuid = Some(Uuid::new_v4());
        to.uuid = Some(Uuid::new_v4());

        from.identity = "d".to_string();
        from.platform = Platform::Ethereum;

        to.identity = "d.bit".to_string();
        to.platform = Platform::Dotbit;

        let mut hold = Hold::default();
        hold.uuid = Uuid::new_v4();
        hold.source = DataSource::Dotbit;

        create_identity_to_identity_hold_record(&client, &from, &to, &hold).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_create_i2i_resolve_record() -> Result<(), Error> {
        let client = make_http_client();
        let mut from = Identity::default();
        let mut to = Identity::default();
        from.uuid = Some(Uuid::new_v4());
        to.uuid = Some(Uuid::new_v4());

        from.identity = "d".to_string();
        from.platform = Platform::Ethereum;

        to.identity = "d.bit".to_string();
        to.platform = Platform::Dotbit;

        let mut resolve = Resolve::default();
        resolve.uuid = Uuid::new_v4();
        resolve.system = DomainNameSystem::DotBit;
        resolve.name = "d.bit".to_string();
        resolve.source = DataSource::Dotbit;

        create_identity_domain_resolve_record(&client, &to, &from, &resolve).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_create_i2c_hold_record() -> Result<(), Error> {
        let client = make_http_client();
        let mut identity = Identity::default();
        let mut contract = Contract::default();

        identity.uuid = Some(Uuid::new_v4());
        identity.platform = Platform::Ethereum;
        identity.identity = "d".to_string();
        identity.display_name = Some("ggbound_thlzyx".to_string());

        contract.uuid = Uuid::new_v4();
        contract.category = ContractCategory::ENS;
        contract.chain = Chain::Ethereum;
        contract.address = "0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85".to_string();

        let mut hold = Hold::default();
        hold.uuid = Uuid::new_v4();
        hold.source = DataSource::TheGraph;
        hold.transaction =
            Some("0x565cb9a10198629955f0f3c86124e45a7d1ad8c47c9e8614dea1ed0897092305".to_string());
        hold.id = "maskbook.eth".to_string();

        create_identity_to_contract_hold_record(&client, &identity, &contract, &hold).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_c2i_reverse_resolve_record() -> Result<(), Error> {
        let client = make_http_client();
        let mut identity = Identity::default();
        let mut contract = Contract::default();

        identity.uuid = Some(Uuid::new_v4());
        identity.platform = Platform::Ethereum;
        identity.identity = "d".to_string();
        identity.display_name = Some("ggbound_thlzyx".to_string());

        contract.uuid = Uuid::new_v4();
        contract.category = ContractCategory::ENS;
        contract.chain = Chain::Ethereum;
        contract.address = "0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85".to_string();

        let mut resolve = Resolve::default();
        resolve.uuid = Uuid::new_v4();
        resolve.system = DomainNameSystem::ENS;
        resolve.name = "maskbook.eth".to_string();
        resolve.source = DataSource::TheGraph;
        create_contract_to_identity_resolve_record(&client, &contract, &identity, &resolve).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_domain() -> Result<(), Error> {
        let client = make_http_client();
        if let Some(found) =
            Resolve::find_by_name_system(&client, "tinpeiling.eth", &DomainNameSystem::ENS).await?
        {
            println!("domain = {:?}", found);
            let json_raw =
                serde_json::to_string(&found).map_err(|err| Error::JSONParseError(err))?;
            println!("domain: {}", json_raw);
        } else {
            return Err(Error::NoResult);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_query_holder() -> Result<(), Error> {
        let client = make_http_client();
        if let Some(found) = Hold::find_by_id_chain_address(
            &client,
            "maskbook.eth",
            &Chain::Ethereum,
            "0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85",
        )
        .await?
        {
            println!("holder = {:?}", found);
            let json_raw =
                serde_json::to_string(&found).map_err(|err| Error::JSONParseError(err))?;
            println!("holder: {}", json_raw);
        } else {
            return Err(Error::NoResult);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_query_nfts() -> Result<(), Error> {
        let client = make_http_client();
        if let Some(found) =
            Identity::find_by_platform_identity(&client, &Platform::Ethereum, "d").await?
        {
            println!("found = {:?}", found);
            let nfts = found
                .nfts(
                    &client,
                    Some(vec![ContractCategory::ENS, ContractCategory::ERC721]),
                    100,
                    0,
                )
                .await?;
            let json_raw =
                serde_json::to_string(&nfts).map_err(|err| Error::JSONParseError(err))?;
            println!("nfts: {}", json_raw);
        } else {
            return Err(Error::NoResult);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_neighbors_with_traversal() -> Result<(), Error> {
        let client = make_http_client();
        if let Some(found) =
            Identity::find_by_platform_identity(&client, &Platform::Ethereum, "d").await?
        {
            println!("found = {:?}", found);
            let edges = found.neighbors_with_traversal(&client, 1).await?;
            let json_raw =
                serde_json::to_string(&edges).map_err(|err| Error::JSONParseError(err))?;
            println!("neighbors: {}", json_raw);
        } else {
            return Err(Error::NoResult);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_neighbors_with_source() -> Result<(), Error> {
        let client = make_http_client();
        if let Some(found) =
            Identity::find_by_platform_identity(&client, &Platform::Ethereum, "d").await?
        {
            println!("found = {:?}", found);
            let edges = found.neighbors(&client, 3).await?;
            let json_raw =
                serde_json::to_string(&edges).map_err(|err| Error::JSONParseError(err))?;
            println!("neighbors_with_source: {}", json_raw);
        } else {
            return Err(Error::NoResult);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_t() -> Result<(), Error> {
        let json_string = r###"
        {
            "error":false,
            "message":"",
            "results":[
                {
                  "edges": [
                    {
                      "attributes": {
                        "created_at": "1970-01-01 00:00:00",
                        "fetcher": "relation_service",
                        "id": "",
                        "source": "nextid",
                        "transaction": "",
                        "updated_at": "2023-05-06 06:12:52",
                        "uuid": "e3857452-6c2e-427c-a8c9-6f5d430c0884"
                      },
                      "directed": true,
                      "discriminator": "nextid",
                      "e_type": "Hold_Identity",
                      "from_id": "ethereum,d",
                      "from_type": "Identities",
                      "to_id": "dotbit,e",
                      "to_type": "Identities"
                    },
                    {
                      "attributes": {
                        "created_at": "1970-01-01 00:00:00",
                        "fetcher": "relation_service",
                        "level": 5,
                        "record_id": "",
                        "source": "nextid",
                        "updated_at": "2023-05-06 06:09:51",
                        "uuid": "68ef6fb8-ddeb-43c4-bad1-0112141ec8c7"
                      },
                      "directed": true,
                      "discriminator": "nextid",
                      "e_type": "Proof_Backward",
                      "from_id": "nextid,b",
                      "from_type": "Identities",
                      "to_id": "ethereum,d",
                      "to_type": "Identities"
                    },
                    {
                      "attributes": {
                        "created_at": "1970-01-01 00:00:00",
                        "fetcher": "relation_service",
                        "level": 5,
                        "record_id": "",
                        "source": "nextid",
                        "updated_at": "2023-05-06 06:09:51",
                        "uuid": "68ef6fb8-ddeb-43c4-bad1-0112141ec8c7"
                      },
                      "directed": true,
                      "discriminator": "nextid",
                      "e_type": "Proof_Forward",
                      "from_id": "ethereum,d",
                      "from_type": "Identities",
                      "to_id": "nextid,b",
                      "to_type": "Identities"
                    }
                  ],
                  "vertices": [
                    "dotbit,e",
                    "nextid,b"
                  ]
                }
              ],
            "version":{
                "api":"v2",
                "edition":"enterprise",
                "schema":2
            }
        }
        "###;
        let record: NeighborsResponse = serde_json::from_str(json_string)?;
        println!("{:?}", record);
        Ok(())
    }
}
