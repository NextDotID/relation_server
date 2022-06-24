mod tests;

use crate::config::C;
use crate::graph::{Edge, Vertex};
use crate::upstream::{Connection, DataSource, Fetcher, Platform};
use crate::util::{make_client, naive_now, parse_body};
use crate::{
    error::Error,
    graph::{edge::Proof, new_db_connection, vertex::Identity},
};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime};
use futures::future::join_all;
use uuid::Uuid;
use gql_client::Client;
use serde::{Deserialize, Serialize};

pub struct Knn3 {
    pub account: String,
}

#[derive(Deserialize, Debug)]
pub struct Ens {
   ens: Vec<String>
}

#[derive(Deserialize, Debug)]
pub struct Data {
   addrs: Vec<Ens>
}

#[derive(Serialize)]
pub struct Vars {
   addr: String
}

#[async_trait]
impl Fetcher for Knn3 {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let query = r#"
                query EnsByAddressQuery($addr: String!){
                    addrs(where: { address: $addr }) {
                    ens
                }
            }
        "#;

        let client = Client::new(C.upstream.knn3_service.url);
        let vars = Vars { addr: "0xd8da6bf26964af9d7eed9e03e53415d37aa96045".to_string() };
        let data = client.query_with_vars::<Data, Vars>(query2, vars).await.unwrap();
        let res = data.unwrap();
        //let data = res.unwrap();
        println!("{:?}", res.addrs.first().unwrap().ens.first().unwrap());
       
        let parse_body = Vec::new();
        Ok(parse_body)
    }
    
    fn ability() -> Vec<(Platform, Vec<Platform>)> {
        let x: (Platform, Vec<Platform>) = (Platform::Ethereum, vec![Platform::Twitter]);
        let mut vec = Vec::new();
        vec.push(x);
        return vec;
    }
}
