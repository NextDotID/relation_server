use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, PartOfCollection, Resolve, Wrapper, HOLD_IDENTITY, PART_OF_COLLECTION, RESOLVE,
};
use crate::tigergraph::vertex::{DomainCollection, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{DataFetcher, DataSource, DomainNameSystem, DomainSearch, Platform, EXT};
use crate::util::{naive_now, option_timestamp_to_naive};
use async_trait::async_trait;
use gql_client::Client as GQLClient;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Serialize)]
struct QueryVars {
    name: String,
}

const UNKNOWN_OWNER: &str = "0x0000000000000000000000000000000000000000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExactMatchResponse {
    domains: ExactMatch,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExactMatch {
    #[serde(rename = "exactMatch")]
    exact_match: Vec<ExactMatchItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExactMatchItem {
    name: String,
    #[serde(rename = "tokenId")]
    token_id: String,
    owner: String,
    #[serde(rename = "expirationDate")]
    expiration_date: i64,
    network: i32, // eth=0, bnb=1
    tld: TldInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TldInfo {
    #[serde(rename = "tldName")]
    tld_name: String,
    #[serde(rename = "chainID")]
    chain_id: String,
}

const QUERY_EXACT_MATCH: &str = r#"
  query domainsByName($name: String!) {
    domains(
      input: {query: $name, first: 200, tldID: 1, domainStatuses: [REGISTERED, UNREGISTERED], buyNow: 0, isVerified: true}
    ) {
      exactMatch {
        name
        tokenId
        owner
        expirationDate
        network
        image
        tld {
          tldName
          chainID
        }
      }
    }
  }
"#;

pub struct SpaceIdV3 {}

#[async_trait]
impl DomainSearch for SpaceIdV3 {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        let mut process_name = name.to_string();
        if name.contains(".") {
            process_name = name.split(".").next().unwrap_or("").to_string();
        }
        if process_name == "".to_string() {
            warn!("SpaceIdV3 domain_search(name='') is not a valid handle name");
            return Ok(vec![]);
        }
        debug!("SpaceIdV3 domain_search(name={})", process_name);

        let exact_items = domain_search(&process_name).await?;
        if exact_items.is_empty() {
            return Ok(vec![]);
        }

        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            label: process_name.clone(),
            updated_at: naive_now(),
        };

        for item in exact_items.iter() {
            let owner = item.owner.clone();
            if owner == "".to_string() || owner.to_lowercase() == UNKNOWN_OWNER {
                continue; // not exist
            }
            let expiration_date: Option<i64> = match item.expiration_date {
                0 => None,          // If the expiration date is 0, return None
                date => Some(date), // Otherwise, return Some(date)
            };
            let expired_at_naive = option_timestamp_to_naive(expiration_date, 0);
            let tld_name = item.tld.tld_name.clone();
            let tld: EXT = tld_name.parse()?;
            if tld == EXT::Gno || tld == EXT::Eth {
                continue; // EXT(`.eth`, `.gno`) are in special upstreams, do not repeated
            }
            if tld == EXT::Unknown {
                continue;
            }

            let domain_name = format!("{}.{}", item.name, tld);
            let domain_platform: Platform = tld.into();
            let domain_system: DomainNameSystem = tld.into();

            // check if platform and system are valid
            if domain_platform == Platform::Unknown {
                continue;
            }
            if domain_system == DomainNameSystem::Unknown {
                continue;
            }

            let addr: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: owner.to_lowercase(),
                uid: None,
                created_at: None,
                display_name: None,
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };

            let domain = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: domain_platform.clone(),
                identity: domain_name.clone(),
                uid: None,
                created_at: None,
                display_name: Some(domain_name.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: expired_at_naive,
                reverse: Some(false),
            };

            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                transaction: Some("".to_string()),
                id: item.token_id.clone(),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: expired_at_naive,
            };
            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                system: domain_system.clone(),
                name: domain_name.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };

            let collection_edge = PartOfCollection {
                system: domain_system.to_string(),
                name: domain_name.clone(),
                tld: tld.to_string(),
                status: "taken".to_string(),
            };

            // hold record
            let hd = hold.wrapper(&addr, &domain, HOLD_IDENTITY);
            // 'regular' resolution involves mapping from a name to an address.
            let rs = resolve.wrapper(&domain, &addr, RESOLVE);
            // create collection edge
            let c = collection_edge.wrapper(&domain_collection, &domain, PART_OF_COLLECTION);

            edges.push(EdgeWrapperEnum::new_hold_identity(hd));
            edges.push(EdgeWrapperEnum::new_resolve(rs));
            edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
        }

        Ok(edges)
    }
}

// Search for ExactDomain
async fn domain_search(name: &str) -> Result<Vec<ExactMatchItem>, Error> {
    let client = GQLClient::new(&C.upstream.spaceid_api.graphql);
    let query = QUERY_EXACT_MATCH.to_string();
    let vars = QueryVars {
        name: name.to_string(),
    };
    let resp = client.query_with_vars::<ExactMatchResponse, QueryVars>(&query, vars);
    let data: Option<ExactMatchResponse> =
        match tokio::time::timeout(std::time::Duration::from_secs(5), resp).await {
            Ok(resp) => match resp {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(
                        "SpaceIdV3 exactdomain_search(name={}): Failed to fetch err: {}",
                        name, err
                    );
                    None
                }
            },
            Err(_) => {
                warn!(
                    "SpaceIdV3 exactdomain_search(name={}) timeout: no response in 5 seconds.",
                    name
                );
                None
            }
        };

    if data.is_none() {
        debug!("SpaceIdV3 exactdomain_search(name={}): No result", name);
        return Ok(vec![]);
    }

    Ok(data.unwrap().domains.exact_match)
}
