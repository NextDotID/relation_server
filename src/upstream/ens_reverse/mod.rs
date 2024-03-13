#[cfg(test)]
mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::Resolve;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_reverse_resolve_record;
use crate::tigergraph::vertex::{Contract, Identity};
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
        if reverse_ens == "" {
            return Ok(vec![]);
        }
        info!("ENS Reverse record: {} => {}", wallet, reverse_ens);

        let mut identity = Identity::default();
        identity.uuid = Some(Uuid::new_v4());
        identity.platform = Platform::Ethereum;
        identity.identity = wallet.clone();
        identity.display_name = Some(reverse_ens.clone());
        let cli = make_http_client();
        identity.create_or_update(&cli).await?;

        // If reverse lookup record reverse_record is None
        // Do not save the reverse_resolve_record
        if record.reverse_record.is_none() {
            return Ok(vec![]);
        }
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
        create_identity_domain_reverse_resolve_record(&cli, &identity, &ens_domain, &resolve)
            .await?;
        create_identity_to_contract_reverse_resolve_record(&cli, &identity, &contract, &resolve)
            .await?;
        Ok(vec![])
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
