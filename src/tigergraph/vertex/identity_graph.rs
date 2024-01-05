use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::{SocialFollow, SocialGraph},
        vertex::IdentityRecord,
    },
    upstream::{ContractCategory, DataSource, DomainNameSystem, Platform},
};
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraph {
    pub graph_id: Option<Uuid>,
    pub vertices: Vec<IdentityRecord>,
    pub edges: Vec<IdentityGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGraphEdge {
    pub source: DataSource,
    pub transaction: Option<String>,
    pub from: IdentityRecord,
    pub to: IdentityRecord,
}

impl IdentityGraph {
    async fn find_by_graph_id(
        client: &Client<HttpConnector>,
        graph_id: Uuid,
    ) -> Result<Option<IdentityGraph>, Error> {
        todo!()
    }

    pub async fn find_by_platform_identity(
        client: &Client<HttpConnector>,
        platform: &Platform,
        identity: &str,
    ) -> Result<Option<IdentityGraph>, Error> {
        todo!()
    }

    pub async fn follow_relation(
        &self,
        client: &Client<HttpConnector>,
        hop: u16,
        follow_type: &str,
    ) -> Result<Option<SocialGraph>, Error> {
        todo!()
    }
}
