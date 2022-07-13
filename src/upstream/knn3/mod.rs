mod tests;

use crate::config::C;
use crate::graph::edge::Own;
use crate::graph::vertex::{nft::Chain, nft::NFTCategory, NFT};
use crate::graph::{Edge, Vertex};
use crate::upstream::{DataSource, Fetcher, Platform, TargetProcessedList};
use crate::util::naive_now;
use crate::{
    error::Error,
    graph::{new_db_connection, vertex::Identity},
};
use async_trait::async_trait;
use gql_client::Client;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Target;

#[derive(Deserialize, Debug)]
pub struct Ens {
    ens: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    addrs: Vec<Ens>,
}

#[derive(Serialize)]
pub struct Vars<'a> {
    addr: &'a str,
}

pub struct Knn3 {}

#[async_trait]
impl Fetcher for Knn3 {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        let query = r#"
            query EnsByAddressQuery($addr: String!){
                addrs(where: { address: $addr }) {
                ens
            }
        }
    "#;
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        let identity = target.identity()?;

        let client = Client::new(C.upstream.knn3_service.url.clone());
        // TODO: Does KNN3 case-sensitive?
        let vars = Vars { addr: &identity };

        let resp = client.query_with_vars::<Data, Vars>(query, vars).await;
        if resp.is_err() {
            warn!(
                "KNN3 fetch | Failed to fetch addrs: {}, err: {:?}",
                identity,
                resp.err()
            );
            return Ok(vec![]);
        }

        let res = resp.unwrap().unwrap();
        if res.addrs.is_empty() {
            info!("KNN3 fetch | address: {} cannot find any result", identity);
            return Ok(vec![]);
        }

        let ens_vec = res.addrs.first().unwrap();
        let db = new_db_connection().await?;

        for ens in ens_vec.ens.iter() {
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: identity.to_lowercase(),
                created_at: None,
                display_name: identity.to_lowercase(),
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
        Ok(ens_vec
            .ens
            .into_iter()
            .map(|ens| Target::NFT(Chain::Ethereum, NFTCategory::ENS, ens))
            .collect())
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}
