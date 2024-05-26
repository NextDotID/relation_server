#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    HyperEdge, Resolve, Wrapper, HYPER_EDGE, REVERSE_RESOLVE, REVERSE_RESOLVE_CONTRACT,
};
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_reverse_resolve_record;
use crate::tigergraph::upsert::create_isolated_vertex;
use crate::tigergraph::vertex::{Contract, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem};
use crate::util::{make_client, make_http_client, naive_now, parse_body, request_with_timeout};
use async_trait::async_trait;
use hyper::{Body, Method};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use super::{Fetcher, Platform, Target, TargetProcessedList};

#[derive(Deserialize, Debug, Clone)]
pub struct Response {
    #[serde(rename = "reverseRecord")]
    pub reverse_record: Option<String>,
    #[allow(unused)]
    pub domains: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ENSReverseLookup {}

#[async_trait]
impl Fetcher for ENSReverseLookup {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }
        let wallet = target.identity().unwrap().to_lowercase();
        let record = fetch_record(&wallet).await?;
        // If reverse lookup record is reset to empty by user,
        // our cache should also be cleared.
        // Reach this by setting `display_name` into `Some("")`.
        let reverse_ens = record.reverse_record.clone().unwrap_or("".into());
        let mut eth_identity = Identity::default();
        eth_identity.uuid = Some(Uuid::new_v4());
        eth_identity.platform = Platform::Ethereum;
        eth_identity.identity = wallet.clone();
        eth_identity.display_name = Some(reverse_ens.clone());
        let cli = make_http_client();
        create_isolated_vertex(&cli, &eth_identity).await?;
        if reverse_ens == "" {
            return Ok(vec![]);
        }
        info!("ENS Reverse record: {} => {}", wallet, reverse_ens);

        eth_identity.reverse = Some(true); // ethereum and primary ens remain same value
        let ens_domain = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::ENS,
            identity: reverse_ens.clone(),
            uid: None,
            created_at: None,
            display_name: Some(reverse_ens.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(true),
        };
        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::ENS,
            address: ContractCategory::ENS.default_contract_address().unwrap(),
            chain: Chain::Ethereum,
            symbol: None,
            updated_at: naive_now(),
        };

        let resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::TheGraph,
            system: DomainNameSystem::ENS,
            name: reverse_ens.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        // reverse resolve record
        create_identity_domain_reverse_resolve_record(&cli, &eth_identity, &ens_domain, &resolve)
            .await?;
        create_identity_to_contract_reverse_resolve_record(
            &cli,
            &eth_identity,
            &contract,
            &resolve,
        )
        .await?;
        Ok(vec![])
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }

        let wallet = target.identity().unwrap().to_lowercase();
        let record = fetch_record(&wallet).await?;
        // If reverse lookup record is reset to empty by user,
        // our cache should also be cleared.
        // Reach this by setting `display_name` into `Some("")`.
        let reverse_ens = record.reverse_record.clone().unwrap_or("".into());
        let mut eth_identity = Identity::default();
        eth_identity.uuid = Some(Uuid::new_v4());
        eth_identity.platform = Platform::Ethereum;
        eth_identity.identity = wallet.clone();
        eth_identity.display_name = Some(reverse_ens.clone());

        let mut edges = EdgeList::new();
        let hv = IdentitiesGraph::default();

        if reverse_ens == "" {
            // if ens reverse is empty, we should also saving the ethereum into identity_graph, create a isolated vertex
            edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
                &hv,
                &eth_identity,
                HYPER_EDGE,
            )));
            info!(?target, "ENS Reverse record is null");
            return Ok((vec![], edges));
        }
        info!(?target, "ENS Reverse record: {} => {}", wallet, reverse_ens);

        eth_identity.reverse = Some(true); // ethereum and primary ens remain same value
        let ens_domain = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::ENS,
            identity: reverse_ens.clone(),
            uid: None,
            created_at: None,
            display_name: Some(reverse_ens.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(true),
        };
        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::ENS,
            address: ContractCategory::ENS.default_contract_address().unwrap(),
            chain: Chain::Ethereum,
            symbol: None,
            updated_at: naive_now(),
        };

        let reverse = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::TheGraph,
            system: DomainNameSystem::ENS,
            name: reverse_ens.clone(),
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        // create reverse resolve record
        let rr = reverse.wrapper(&eth_identity, &ens_domain, REVERSE_RESOLVE);
        let rrc = reverse.wrapper(&eth_identity, &contract, REVERSE_RESOLVE_CONTRACT);
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &eth_identity,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &ens_domain,
            HYPER_EDGE,
        )));
        edges.push(EdgeWrapperEnum::new_reverse_resolve(rr));
        edges.push(EdgeWrapperEnum::new_reverse_resolve_contract(rrc));

        Ok((vec![], edges))
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Ethereum])
    }
}

pub async fn fetch_record(wallet: &str) -> Result<Response, Error> {
    let client = make_client();
    let url: http::Uri = format!("{}{}", C.upstream.ens_reverse.url, wallet)
        .parse()
        .map_err(|err: http::uri::InvalidUri| {
            Error::ParamError(format!("URI Format error: {}", err))
        })?;

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(url)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("ENSReverse Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "ENSReverse fetch | fetch_record error: {:?}",
                err.to_string()
            ))
        })?;

    if !resp.status().is_success() {
        return Err(Error::General(
            format!("ENSReverse fetch Error: {}", resp.status()),
            resp.status(),
        ));
    }
    parse_body(&mut resp).await
}
