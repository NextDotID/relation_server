mod tests;

use crate::config::C;
use crate::graph::edge::Own;
use crate::graph::vertex::{nft::Chain, nft::NFTCategory, NFT};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, IdentityProcessList, Platform};
use crate::util::naive_now;
use crate::{
    error::Error,
    graph::{new_db_connection, vertex::Identity},
};
use async_trait::async_trait;
use gql_client::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

pub struct Knn3 {
    pub platform: String,
    pub identity: String,
}

#[async_trait]
impl Fetcher for Knn3 {
    async fn fetch(&self) -> Result<IdentityProcessList, Error> {
        let query = r#"
            query EnsByAddressQuery($addr: String!){
                addrs(where: { address: $addr }) {
                ens
            }
        }
    "#;

        let client = Client::new(C.upstream.knn3_service.url.clone());
        let vars = Vars {
            addr: self.identity.clone(),
        };

        let data = client
            .query_with_vars::<Data, Vars>(query, vars)
            .await
            .unwrap();

        let res = data.unwrap();
        if res.addrs.first().is_none() {
            return Ok(IdentityProcessList::new());
        }

        let ens_vec = res.addrs.first().unwrap();
        let db = new_db_connection().await?;

        for ens in ens_vec.ens.iter() {
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: self.identity.clone().to_lowercase(),
                created_at: None,
                display_name: self.identity.clone().to_lowercase(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let from_record = from.create_or_update(&db).await?;

            let to: NFT = NFT {
                uuid: Uuid::new_v4(),
                category: NFTCategory::ENS,
                contract: NFTCategory::ENS.default_contract_address().unwrap(),
                id: ens.to_string(),
                chain: Chain::Ethereum,
                symbol: None,
                updated_at: naive_now(),
            };
            let to_record = to.create_or_update(&db).await?;

            let ownership: Own = Own {
                uuid: Uuid::new_v4(),
                transaction: None,
                source: DataSource::Knn3,
            };
            ownership.connect(&db, &from_record, &to_record).await?;
        }
        Ok(vec![])
    }

    fn ability(&self) -> Vec<(Vec<Platform>, Vec<Platform>)> {
        return vec![(vec![Platform::Ethereum], vec![])];
    }
}
