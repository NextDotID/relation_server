pub mod hold;
pub mod proof;
pub mod resolve;
pub use hold::{Hold, HoldRecord, HOLD_CONTRACT, HOLD_IDENTITY};
pub use proof::{
    Proof, ProofRecord, EDGE_NAME as PROOF_EDGE, REVERSE_EDGE_NAME as PROOF_REVERSE_EDGE,
};
pub use resolve::{Resolve, ResolveRecord, RESOLVE, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT};

use crate::{
    config::C,
    error::Error,
    tigergraph::{
        vertex::{Identity, IdentityRecord, Vertex},
        Attribute, EdgeWrapper,
    },
};

use async_graphql::Union;
use async_trait::async_trait;
use http::uri::InvalidUri;
use hyper::Method;
use hyper::{client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::de::{self, Deserialize as DeDeserialize, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// DeserializeOwned + Serialize + Clone
/// All `Edge` records.
/// // #[typetag::serde]
#[async_trait]
pub trait Edge<Source, Target, RecordType>
where
    Self: Sized + DeserializeOwned + Serialize + Clone,
    Source: Sized + Vertex,
    Target: Sized + Vertex,
    RecordType: Sized + DeserializeOwned + Serialize + Clone,
{
    fn e_type(&self) -> String;
    fn directed(&self) -> bool;

    async fn connect(
        &self,
        client: &Client<HttpConnector>,
        from: &Source,
        to: &Target,
    ) -> Result<(), Error>;
}

pub trait Wrapper<RecordType, Source, Target>
where
    Self: Sized + DeserializeOwned + Serialize + Clone,
    Source: Sized + Vertex,
    Target: Sized + Vertex,
    RecordType: Sized + DeserializeOwned + Serialize + Clone,
{
    fn wrapper(
        &self,
        from: &Source,
        to: &Target,
        name: &str,
    ) -> EdgeWrapper<RecordType, Source, Target>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeRecord<T> {
    /// The `e_type` field of `TigerGraph` is <edge_type>.
    pub e_type: String,

    /// The `directed` field of `TigerGraph` is edge is directed - true / undirected - false.
    pub directed: bool,

    /// The `from_id` field of `TigerGraph` is <source-vertex-id>
    pub from_id: String,

    /// The `from_type` field of `TigerGraph` is <source-vertex-type>
    pub from_type: String,

    /// The `to_id` field of `TigerGraph` is <target-vertex-id>
    pub to_id: String,

    /// The `to_type` field of `TigerGraph` is <target-vertex-type>
    pub to_type: String,

    /// A discriminator is an attribute or a set of attributes that can be used to uniquely identify an edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,

    /// The attributes of <edge>
    pub attributes: T,
}

// Define a custom trait with a function that takes multiple input parameters.
pub trait FromWithParams<T> {
    fn from_with_params(
        e_type: String,
        directed: bool,
        from_id: String,
        from_type: String,
        to_id: String,
        to_type: String,
        attributes: T,
    ) -> Self;
}

#[derive(Union, Serialize, Debug, Clone)]
pub enum EdgeUnion {
    HoldRecord(HoldRecord),
    ProofRecord(ProofRecord),
}

// match record {
//     EdgeUnion::HoldRecord(hold_record) => {
//         println!("Hold record: {:?}", hold_record)
//     }
//     EdgeUnion::ProofRecord(proof_record) => {
//         println!("Proof record: {:?}", proof_record)
//     }
// }

impl<'de> Deserialize<'de> for EdgeUnion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EdgeUnionVisitor;

        impl<'de> Visitor<'de> for EdgeUnionVisitor {
            type Value = EdgeUnion;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("HoldRecord or ProofRecord")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut e_type: Option<String> = None;
                let mut directed: Option<bool> = None;
                let mut from_id: Option<String> = None;
                let mut from_type: Option<String> = None;
                let mut to_id: Option<String> = None;
                let mut to_type: Option<String> = None;
                let mut discriminator: Option<String> = None;
                let mut attributes: Option<serde_json::Value> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "e_type" => {
                            if e_type.is_none() {
                                e_type = Some(map.next_value()?);
                            }
                        }
                        "directed" => {
                            if directed.is_none() {
                                directed = Some(map.next_value()?);
                            }
                        }
                        "from_id" => {
                            if from_id.is_none() {
                                from_id = Some(map.next_value()?);
                            }
                        }
                        "from_type" => {
                            if from_type.is_none() {
                                from_type = Some(map.next_value()?);
                            }
                        }
                        "to_id" => {
                            if to_id.is_none() {
                                to_id = Some(map.next_value()?);
                            }
                        }
                        "to_type" => {
                            if to_type.is_none() {
                                to_type = Some(map.next_value()?);
                            }
                        }
                        "discriminator" => {
                            if discriminator.is_none() {
                                discriminator = Some(map.next_value()?);
                            }
                        }
                        "attributes" => {
                            if attributes.is_none() {
                                attributes = Some(map.next_value()?);
                            }
                            // } else {
                            //     return Err(de::Error::custom("duplicate attributes"));
                            // }
                        }
                        _ => {
                            return Err(de::Error::custom("unexpected field"));
                        }
                    }
                }

                match (e_type, attributes) {
                    (Some(e_type), Some(attributes)) => {
                        if e_type == "Proof_Forward" || e_type == "Proof_Backward" {
                            let proof: Proof =
                                serde_json::from_value(attributes).map_err(de::Error::custom)?;
                            Ok(EdgeUnion::ProofRecord(ProofRecord(
                                EdgeRecord::from_with_params(
                                    e_type,
                                    directed.unwrap_or_default(),
                                    from_id.unwrap_or_default(),
                                    from_type.unwrap_or_default(),
                                    to_id.unwrap_or_default(),
                                    to_type.unwrap_or_default(),
                                    proof,
                                ),
                            )))
                        } else if e_type == "Hold_Identity" {
                            let hold: Hold =
                                serde_json::from_value(attributes).map_err(de::Error::custom)?;
                            Ok(EdgeUnion::HoldRecord(HoldRecord(
                                EdgeRecord::from_with_params(
                                    e_type,
                                    directed.unwrap_or_default(),
                                    from_id.unwrap_or_default(),
                                    from_type.unwrap_or_default(),
                                    to_id.unwrap_or_default(),
                                    to_type.unwrap_or_default(),
                                    hold,
                                ),
                            )))
                        } else {
                            Err(de::Error::unknown_variant(
                                &e_type,
                                &["Proof_Backward", "Proof_Forward", "Hold_Identity"],
                            ))
                        }
                    }
                    _ => Err(de::Error::custom("missing e_type or attributes")),
                }
            }
        }

        deserializer.deserialize_map(EdgeUnionVisitor)
    }
}
