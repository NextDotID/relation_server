use crate::{
    config::C,
    error::Error,
    tigergraph::{
        edge::EdgeUnion,
        vertex::{IdentityGraph, IdentityRecord},
    },
    upstream::{ContractCategory, DataSource, DomainNameSystem, Platform},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialFollow {
    pub source: DataSource,
    pub from: IdentityGraph,
    pub to: IdentityGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraph {
    pub list: Option<Vec<IdentityGraph>>,
    pub topology: Option<Vec<SocialFollow>>,
}
