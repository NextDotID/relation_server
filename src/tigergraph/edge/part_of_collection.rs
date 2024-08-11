use crate::{
    error::Error,
    tigergraph::{
        edge::{Edge, EdgeRecord, EdgeWrapper, FromWithParams, Wrapper},
        vertex::{DomainCollection, Identity, Vertex, VertexRecord},
        Attribute, OpCode, Transfer,
    },
    upstream::{DomainStatus, Platform},
    util::{option_naive_datetime_from_string, option_naive_datetime_to_string},
};

use chrono::NaiveDateTime;
use hyper::{client::HttpConnector, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::value::{Map, Value};
use std::collections::HashMap;
use uuid::Uuid;

// always DomainCollection -> Identities
pub const PART_OF_COLLECTION: &str = "PartOfCollection";
pub const IS_DIRECTED: bool = true;

/// PartOfCollection
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct PartOfCollection {
    /// Domain Name system
    pub platform: Platform,
    /// Name of domain (e.g., `vitalik.eth`)
    pub name: String,
    /// Extension of domain (e.g. eth)
    pub tld: String,
    /// Status of domain
    pub status: DomainStatus,
}

impl Default for PartOfCollection {
    fn default() -> Self {
        Self {
            platform: Default::default(),
            name: Default::default(),
            tld: Default::default(),
            status: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartOfCollectionRecord(pub EdgeRecord<PartOfCollection>);

impl FromWithParams<PartOfCollection> for EdgeRecord<PartOfCollection> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: PartOfCollection,
    ) -> Self {
        EdgeRecord {
            e_type,
            directed,
            from_id,
            from_type,
            to_id,
            to_type,
            discriminator: None,
            attributes,
        }
    }
}

impl From<EdgeRecord<PartOfCollection>> for PartOfCollectionRecord {
    fn from(record: EdgeRecord<PartOfCollection>) -> Self {
        PartOfCollectionRecord(record)
    }
}

impl std::ops::Deref for PartOfCollectionRecord {
    type Target = EdgeRecord<PartOfCollection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PartOfCollectionRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for EdgeRecord<PartOfCollection> {
    type Target = PartOfCollection;

    fn deref(&self) -> &Self::Target {
        &self.attributes
    }
}

impl std::ops::DerefMut for EdgeRecord<PartOfCollection> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attributes
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartOfCollectionAttribute(HashMap<String, Attribute>);

// Implement the `From` trait for converting `PartOfCollectionRecord` into a `HashMap<String, Attr>`.
impl Transfer for PartOfCollectionRecord {
    fn to_attributes_map(&self) -> HashMap<String, Attribute> {
        let mut attributes_map = HashMap::new();
        attributes_map.insert(
            "platform".to_string(),
            Attribute {
                value: json!(self.attributes.platform.to_string()),
                op: None,
            },
        );
        attributes_map.insert(
            "name".to_string(),
            Attribute {
                value: json!(self.attributes.name),
                op: Some(OpCode::IgnoreIfExists),
            },
        );
        attributes_map.insert(
            "tld".to_string(),
            Attribute {
                value: json!(self.attributes.tld.to_string()),
                op: None,
            },
        );
        attributes_map.insert(
            "status".to_string(),
            Attribute {
                value: json!(self.attributes.status.to_string()),
                op: None,
            },
        );

        attributes_map
    }

    fn to_json_value(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("platform".to_string(), json!(self.platform));
        map.insert("name".to_string(), json!(self.name));
        map.insert("tld".to_string(), json!(self.tld));
        map.insert("status".to_string(), json!(self.status));
        map
    }
}

#[async_trait::async_trait]
impl Edge<DomainCollection, Identity, PartOfCollectionRecord> for PartOfCollectionRecord {
    fn e_type(&self) -> String {
        self.e_type.clone()
    }

    fn directed(&self) -> bool {
        // TODO: query from server is the best solution
        self.directed.clone()
    }

    /// Find an edge by UUID.
    async fn find_by_uuid(
        _client: &Client<HttpConnector>,
        _uuid: &Uuid,
    ) -> Result<Option<PartOfCollectionRecord>, Error> {
        todo!()
    }

    /// Find `EdgeRecord` by source and target
    async fn find_by_from_to(
        &self,
        _client: &Client<HttpConnector>,
        _from: &VertexRecord<DomainCollection>,
        _to: &VertexRecord<Identity>,
        _filter: Option<HashMap<String, String>>,
    ) -> Result<Option<Vec<PartOfCollectionRecord>>, Error> {
        todo!()
    }

    /// Connect 2 vertex.
    async fn connect(
        &self,
        _client: &Client<HttpConnector>,
        _from: &DomainCollection,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }

    /// notice this function is deprecated
    async fn connect_reverse(
        &self,
        _client: &Client<HttpConnector>,
        _from: &DomainCollection,
        _to: &Identity,
    ) -> Result<(), Error> {
        todo!()
    }
}

impl Wrapper<PartOfCollectionRecord, DomainCollection, Identity> for PartOfCollection {
    fn wrapper(
        &self,
        from: &DomainCollection,
        to: &Identity,
        name: &str,
    ) -> EdgeWrapper<PartOfCollectionRecord, DomainCollection, Identity> {
        let part_of = EdgeRecord::from_with_params(
            name.to_string(),
            IS_DIRECTED,
            from.primary_key(),
            from.vertex_type(),
            to.primary_key(),
            to.vertex_type(),
            self.to_owned(),
        );
        EdgeWrapper {
            edge: PartOfCollectionRecord(part_of),
            source: from.to_owned(),
            target: to.to_owned(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AvailableDomain {
    /// Domain Name system (e.g., `ens`)
    pub platform: Platform,
    /// Name of domain (e.g., `vitalik.eth`)
    pub name: String,
    /// Extension of domain
    pub tld: String,
    /// Expiration time
    #[serde(deserialize_with = "option_naive_datetime_from_string")]
    #[serde(serialize_with = "option_naive_datetime_to_string")]
    pub expired_at: Option<NaiveDateTime>,
    /// Availability is `true` means that the domain is available for registration
    /// Availability is `false` means that the domain has taken by some wallet
    pub availability: bool,
    /// DomainStatus: taken/protected/available
    pub status: DomainStatus,
}
