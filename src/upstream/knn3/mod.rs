mod tests;

use crate::config::C;
use crate::graph::edge::Own;
use crate::graph::vertex::{nft::Chain, nft::NFTCategory, NFT};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform};
use crate::util::naive_now;
use crate::{
    error::Error,
    graph::{new_db_connection, vertex::Identity},
};
use async_trait::async_trait;
use gql_client::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct Knn3 {
    pub account: String,
}

#[derive(Deserialize, Debug)]
pub struct Ens {
    ens: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    addrs: Vec<Ens>,
}

#[derive(Serialize)]
pub struct Vars {
    addr: String,
}

#[async_trait]
impl Fetcher for Knn3 {
    async fn fetch(&self) -> Result<(), Error> {
        let query = r#"
                query EnsByAddressQuery($addr: String!){
                    addrs(where: { address: $addr }) {
                    ens
                }
            }
        "#;
        let client = Client::new(C.upstream.knn3_service.url.clone());
        let vars = Vars {
            addr: "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string(),
        };
        let data = client
            .query_with_vars::<Data, Vars>(query, vars)
            .await.unwrap();
        let res = data.unwrap();
        let ens_vec = res.addrs.first().unwrap();

        let db = new_db_connection().await?;

        for ens in ens_vec.ens.iter() {
            //let ens = item.ens.unwrap();
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: self.account.clone(),
                created_at: None,
                display_name: self.account.clone(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let from_record = from.create_or_update(&db).await?;

            let to: NFT = NFT {
                uuid: Uuid::new_v4(),
                category: NFTCategory::ENS,
                contract: "".to_string(),
                id: ens.to_string(),
                chain: Chain::Ethereum,
                symbol: None,
                updated_at: naive_now(),
            };
            let to_record = to.create_or_update(&db).await?;

            let owner_ship: Own = Own {
                uuid: Uuid::new_v4(),
                transaction: None,
                source: DataSource::Knn3,
            };
            owner_ship.connect(&db, &from_record, &to_record).await?;
        }
        Ok(())
    }

    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)> {
        return vec![(
            vec![Platform::Ethereum],
            vec![],
        )];
    }
}
