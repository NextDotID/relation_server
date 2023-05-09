#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::error::Error;
    use crate::tigergraph::{
        create_identity_to_identity_hold_record, create_identity_to_identity_proof_two_way_binding,
    };
    use crate::{
        tigergraph::{
            edge::{proof::Level, Hold, Proof},
            vertex::{Identity, IdentityRecord, NeighborsResponse},
        },
        upstream::{DataFetcher, DataSource, Platform},
        util::make_http_client,
    };

    #[tokio::test]
    async fn test_create_i2i_proof_two_way_binding() -> Result<(), Error> {
        let client = make_http_client();
        let mut from = Identity::default();
        let mut to = Identity::default();
        from.uuid = Some(Uuid::new_v4());
        to.uuid = Some(Uuid::new_v4());

        from.identity = "g".to_string();
        from.display_name = Some("ggbound".to_string());
        to.identity = "f".to_string();

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
        proof_forward.level = Level::VeryConfident;
        proof_backward.level = Level::VeryConfident;
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
        to.identity = "e".to_string();

        from.platform = Platform::Ethereum;
        to.platform = Platform::Dotbit;

        let mut hold = Hold::default();
        hold.uuid = Uuid::new_v4();
        hold.source = DataSource::Dotbit;

        create_identity_to_identity_hold_record(&client, &from, &to, &hold).await?;
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
