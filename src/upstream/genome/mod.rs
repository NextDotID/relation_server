mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, HyperEdge, PartOfCollection, Resolve, Wrapper, HOLD_CONTRACT, HOLD_IDENTITY, HYPER_EDGE,
    PART_OF_COLLECTION, RESOLVE, REVERSE_RESOLVE,
};
use crate::tigergraph::upsert::create_ens_identity_ownership;
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_contract_hold_record;
use crate::tigergraph::vertex::{Contract, DomainCollection, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, DomainSearch, DomainStatus,
    Fetcher, Platform, Target, TargetProcessedList, EXT,
};
use crate::util::{
    make_client, make_http_client, naive_now, parse_body, request_with_timeout, timestamp_to_naive,
};
use async_trait::async_trait;
use http::uri::InvalidUri;
use http::StatusCode;
use hyper::{Body, Method, Request};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaseResponse {
    pub code: i32,
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GetNameResponse {
    #[serde(flatten)]
    base: BaseResponse,
    data: Option<Vec<Metadata>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GetAddressResponse {
    #[serde(flatten)]
    base: BaseResponse,
    data: Option<Vec<Metadata>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Metadata {
    name: String,
    tld_name: String,
    owner: String,
    expired_at: i64,
    is_default: bool,
    token_id: Option<String>,
    image_url: Option<String>,
}

pub struct Genome {}

#[async_trait]
impl Fetcher for Genome {
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error> {
        if !Self::can_fetch(target) {
            return Ok(vec![]);
        }

        match target {
            Target::Identity(platform, identity) => {
                fetch_connections_by_platform_identity(platform, identity).await
            }
            Target::NFT(_, _, _, _) => todo!(),
        }
    }

    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
        if !Self::can_fetch(target) {
            return Ok((vec![], vec![]));
        }
        match target.platform()? {
            Platform::Ethereum => batch_fetch_by_address(target).await,
            Platform::Genome => batch_fetch_by_domain(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::Genome, Platform::Ethereum])
    }
}

async fn batch_fetch_by_address(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let address = target.identity()?.to_lowercase();
    let domains = get_name(&address).await?;
    if domains.is_empty() {
        debug!(?target, "Genome get_name result is empty");
        return Ok((vec![], vec![]));
    }

    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    for d in domains.into_iter() {
        let genome_domain = format!("{}.{}", d.name, d.tld_name);
        let mut token_id = String::from("");
        let expired_at_naive = timestamp_to_naive(d.expired_at, 0);

        if let Some(_token_id) = d.token_id.clone() {
            token_id = _token_id;
        }

        let gno: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Genome,
            identity: genome_domain.clone(),
            uid: None,
            created_at: None,
            display_name: Some(genome_domain.clone()),
            added_at: naive_now(),
            avatar_url: d.image_url.clone(),
            profile_url: None,
            updated_at: naive_now(),
            expired_at: expired_at_naive,
            reverse: Some(d.is_default.clone()),
        };

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: d.owner.clone().to_lowercase(),
            uid: None,
            created_at: None,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(d.is_default.clone()),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            transaction: Some("".to_string()),
            id: token_id,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: expired_at_naive,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            system: DomainNameSystem::Genome,
            name: genome_domain.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::GNS,
            address: ContractCategory::GNS.default_contract_address().unwrap(),
            chain: Chain::Gnosis,
            symbol: Some("GNS".to_string()),
            updated_at: naive_now(),
        };

        if d.is_default {
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                system: DomainNameSystem::Genome,
                name: genome_domain.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };
            debug!("{} => Genome({}) is_default", address, genome_domain);
            let rr = reverse.wrapper(&addr, &gno, REVERSE_RESOLVE);
            edges.push(EdgeWrapperEnum::new_reverse_resolve(rr));
        }

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
        ));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &gno, HYPER_EDGE),
        ));

        // 'regular' resolution involves mapping from a name to an address.
        let rs = resolve.wrapper(&gno, &addr, RESOLVE);
        // hold record
        let hd = hold.wrapper(&addr, &gno, HOLD_IDENTITY);
        let hdc = hold.wrapper(&addr, &contract, HOLD_CONTRACT);
        edges.push(EdgeWrapperEnum::new_resolve(rs));
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
    }

    // after genome, nothing return for next target
    Ok((vec![], edges))
}

async fn batch_fetch_by_domain(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let name = target.identity()?.to_lowercase();
    let name_with_out_tld: &str = name.trim_end_matches(".gno");
    let domains: Vec<Metadata> = get_address(name_with_out_tld).await?; // get_address(domain)
    if domains.is_empty() {
        debug!(?target, "Genome get_address result is empty");
        return Ok((vec![], vec![]));
    }
    let address = domains.first().unwrap().owner.clone();
    let mut next_targets = TargetProcessedList::new();
    next_targets.push(Target::Identity(
        Platform::Ethereum,
        address.clone().to_lowercase(),
    ));

    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    for d in domains.into_iter() {
        let genome_domain = format!("{}.{}", d.name, d.tld_name);
        let mut token_id = String::from("");
        let expired_at_naive = timestamp_to_naive(d.expired_at, 0);

        if let Some(_token_id) = d.token_id.clone() {
            token_id = _token_id;
        }
        let gno: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Genome,
            identity: genome_domain.clone(),
            uid: None,
            created_at: None,
            display_name: Some(genome_domain.clone()),
            added_at: naive_now(),
            avatar_url: d.image_url.clone(),
            profile_url: None,
            updated_at: naive_now(),
            expired_at: expired_at_naive,
            reverse: Some(d.is_default.clone()),
        };

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: d.owner.clone().to_lowercase(),
            uid: None,
            created_at: None,
            display_name: None,
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(d.is_default.clone()),
        };

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            transaction: Some("".to_string()),
            id: token_id,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: expired_at_naive,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            system: DomainNameSystem::Genome,
            name: genome_domain.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::GNS,
            address: ContractCategory::GNS.default_contract_address().unwrap(),
            chain: Chain::Gnosis,
            symbol: Some("GNS".to_string()),
            updated_at: naive_now(),
        };

        if d.is_default {
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                system: DomainNameSystem::Genome,
                name: genome_domain.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };
            debug!("{} => Genome({}) is_default", address, genome_domain);
            let rr = reverse.wrapper(&addr, &gno, REVERSE_RESOLVE);
            edges.push(EdgeWrapperEnum::new_reverse_resolve(rr));
        }

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
        ));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &gno, HYPER_EDGE),
        ));

        // 'regular' resolution involves mapping from a name to an address.
        let rs = resolve.wrapper(&gno, &addr, RESOLVE);
        // hold record
        let hd = hold.wrapper(&addr, &gno, HOLD_IDENTITY);
        let hdc = hold.wrapper(&addr, &contract, HOLD_CONTRACT);
        edges.push(EdgeWrapperEnum::new_resolve(rs));
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        edges.push(EdgeWrapperEnum::new_hold_contract(hdc));
    }

    Ok((next_targets, edges))
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Ethereum => fetch_domain_by_address(platform, identity).await,
        Platform::Genome => fetch_address_by_domain(platform, identity).await,
        _ => Ok(vec![]),
    }
}

async fn fetch_domain_by_address(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let address = identity.to_lowercase();
    let domains = get_name(&address).await?;
    if domains.is_empty() {
        debug!("Genome get_name result is empty");
    }

    for d in domains.into_iter() {
        let genome_domain = format!("{}.{}", d.name, d.tld_name);
        let mut profile_url = String::from("");
        let mut token_id = String::from("");
        let expired_at_naive = timestamp_to_naive(d.expired_at, 0);

        if let Some(_token_id) = d.token_id.clone() {
            token_id = _token_id;
            profile_url = format!(
                "
            https://genomedomains.com/name/14/{}?tldName=gno&name={}",
                token_id,
                d.name.clone()
            );
        }
        let gno: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Genome,
            identity: genome_domain.clone(),
            uid: None,
            created_at: None,
            display_name: Some(genome_domain.clone()),
            added_at: naive_now(),
            avatar_url: d.image_url.clone(),
            profile_url: Some(profile_url),
            updated_at: naive_now(),
            expired_at: expired_at_naive,
            reverse: Some(d.is_default.clone()),
        };

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: d.owner.clone().to_lowercase(),
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

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            transaction: Some("".to_string()),
            id: token_id,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: expired_at_naive,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            system: DomainNameSystem::Genome,
            name: genome_domain.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::GNS,
            address: ContractCategory::GNS.default_contract_address().unwrap(),
            chain: Chain::Gnosis,
            symbol: Some("GNS".to_string()),
            updated_at: naive_now(),
        };

        tracing::debug!("{} => {} created.", address, d.name);
        // create_identity_to_identity_hold_record(&cli, &addr, &gno, &hold).await?;
        // 'regular' resolution involves mapping from a name to an address.
        create_identity_domain_resolve_record(&cli, &gno, &addr, &resolve).await?;

        // ownership create `Hold_Identity` connection but only Wallet connected to HyperVertex
        create_ens_identity_ownership(&cli, &addr, &gno, &hold).await?;
        create_identity_to_contract_hold_record(&cli, &addr, &contract, &hold).await?;

        if d.is_default {
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                system: DomainNameSystem::Genome,
                name: genome_domain.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };
            tracing::debug!("{} => {} is_default: {:?}", address, d.name, d.is_default);
            create_identity_domain_reverse_resolve_record(&cli, &addr, &gno, &reverse).await?;
        }
    }
    // after genome, nothing return for next target
    return Ok(vec![]);
}

async fn fetch_address_by_domain(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let name_with_out_tld: &str = identity.trim_end_matches(".gno");
    let domains: Vec<Metadata> = get_address(name_with_out_tld).await?; // get_address(domain)
    if domains.is_empty() {
        debug!("Genome get_address result is empty");
    }
    let address = domains.first().unwrap().owner.clone();
    for d in domains.into_iter() {
        let genome_domain = format!("{}.{}", d.name, d.tld_name);
        let mut profile_url = String::from("");
        let mut token_id = String::from("");
        let expired_at_naive = timestamp_to_naive(d.expired_at, 0);

        if let Some(_token_id) = d.token_id.clone() {
            token_id = _token_id;
            profile_url = format!(
                "
            https://genomedomains.com/name/14/{}?tldName=gno&name={}",
                token_id,
                d.name.clone()
            );
        }
        let gno: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Genome,
            identity: genome_domain.clone(),
            uid: None,
            created_at: None,
            display_name: Some(genome_domain.clone()),
            added_at: naive_now(),
            avatar_url: d.image_url.clone(),
            profile_url: Some(profile_url),
            updated_at: naive_now(),
            expired_at: expired_at_naive,
            reverse: Some(d.is_default.clone()),
        };

        let addr: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Ethereum,
            identity: d.owner.clone().to_lowercase(),
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

        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            transaction: Some("".to_string()),
            id: token_id,
            created_at: None,
            updated_at: naive_now(),
            fetcher: DataFetcher::DataMgrService,
            expired_at: expired_at_naive,
        };

        let resolve: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            system: DomainNameSystem::Genome,
            name: genome_domain.clone(),
            fetcher: DataFetcher::DataMgrService,
            updated_at: naive_now(),
        };

        let contract = Contract {
            uuid: Uuid::new_v4(),
            category: ContractCategory::GNS,
            address: ContractCategory::GNS.default_contract_address().unwrap(),
            chain: Chain::Gnosis,
            symbol: Some("GNS".to_string()),
            updated_at: naive_now(),
        };

        tracing::debug!("{} => {} created.", address, d.name);
        // create_identity_to_identity_hold_record(&cli, &addr, &gno, &hold).await?;
        // 'regular' resolution involves mapping from a name to an address.
        create_identity_domain_resolve_record(&cli, &gno, &addr, &resolve).await?;

        // ownership create `Hold_Identity` connection but only Wallet connected to HyperVertex
        create_ens_identity_ownership(&cli, &addr, &gno, &hold).await?;
        create_identity_to_contract_hold_record(&cli, &addr, &contract, &hold).await?;

        if d.is_default {
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::SpaceId,
                system: DomainNameSystem::Genome,
                name: genome_domain.clone(),
                fetcher: DataFetcher::DataMgrService,
                updated_at: naive_now(),
            };
            tracing::debug!("{} => {} is_default: {:?}", address, d.name, d.is_default);
            create_identity_domain_reverse_resolve_record(&cli, &addr, &gno, &reverse).await?;
        }
    }

    return Ok(vec![Target::Identity(
        Platform::Ethereum,
        address.clone().to_lowercase(),
    )]);
}

async fn get_name(address: &str) -> Result<Vec<Metadata>, Error> {
    let client = make_client().await.unwrap();
    let uri: http::Uri = format!(
        "{}/get_name?tld=gno&address={}",
        C.upstream.genome_api.url.clone(),
        address
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("Genome get_name Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("Genome get_name | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("Genome get_name error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<GetNameResponse>(&mut resp).await {
        Ok(result) => {
            if result.base.code != 0 {
                let err_message = format!(
                    "Genome get_name error | Code: {:?}, Message: {:?}",
                    result.base.code, result.base.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let r: Vec<Metadata> = result.data.map_or(vec![], |res| res);
            debug!("Genome get_name records found {}.", r.len(),);
            r
        }
        Err(err) => {
            let err_message = format!("Genome get_name error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    Ok(result)
}

async fn get_address(domain: &str) -> Result<Vec<Metadata>, Error> {
    let client = make_client().await.unwrap();
    let uri: http::Uri = format!(
        "{}/get_address?tld=gno&domain={}",
        C.upstream.genome_api.url.clone(),
        domain
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!("Genome get_address Build Request Error {}", _err))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("Genome get_name | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("Genome get_address error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<GetNameResponse>(&mut resp).await {
        Ok(result) => {
            if result.base.code != 0 {
                let err_message = format!(
                    "Genome get_address error | Code: {:?}, Message: {:?}",
                    result.base.code, result.base.msg
                );
                error!(err_message);
                return Err(Error::General(
                    err_message,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            let r: Vec<Metadata> = result.data.map_or(vec![], |res| res);
            debug!("Genome get_address records found {}.", r.len(),);
            r
        }
        Err(err) => {
            let err_message = format!("Genome get_address error parse_body error: {:?}", err);
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };

    Ok(result)
}

#[async_trait]
impl DomainSearch for Genome {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        if name == "" {
            warn!("Genome domain_search(name='') is not a valid domain name");
            return Ok(vec![]);
        }
        debug!("Genome domain_search(name={})", name);

        let domains: Vec<Metadata> = get_address(name).await?; // get_address(domain)
        if domains.is_empty() {
            debug!("Genome get_address(name={}) result is empty", name);
            return Ok(vec![]);
        }
        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            id: name.to_string(),
            updated_at: naive_now(),
        };

        let address = domains.first().unwrap().owner.clone();
        for d in domains.into_iter() {
            if d.name == name.to_string() {
                let genome_domain = format!("{}.{}", d.name, d.tld_name);
                let mut token_id = String::from("");
                let expired_at_naive = timestamp_to_naive(d.expired_at, 0);

                if let Some(_token_id) = d.token_id.clone() {
                    token_id = _token_id;
                }

                let gno: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Genome,
                    identity: genome_domain.clone(),
                    uid: None,
                    created_at: None,
                    display_name: Some(genome_domain.clone()),
                    added_at: naive_now(),
                    avatar_url: d.image_url.clone(),
                    profile_url: None,
                    updated_at: naive_now(),
                    expired_at: expired_at_naive,
                    reverse: Some(d.is_default.clone()),
                };

                let addr: Identity = Identity {
                    uuid: Some(Uuid::new_v4()),
                    platform: Platform::Ethereum,
                    identity: d.owner.clone().to_lowercase(),
                    uid: None,
                    created_at: None,
                    display_name: None,
                    added_at: naive_now(),
                    avatar_url: None,
                    profile_url: None,
                    updated_at: naive_now(),
                    expired_at: None,
                    reverse: Some(d.is_default.clone()),
                };

                let hold: Hold = Hold {
                    uuid: Uuid::new_v4(),
                    source: DataSource::SpaceId,
                    transaction: Some("".to_string()),
                    id: token_id,
                    created_at: None,
                    updated_at: naive_now(),
                    fetcher: DataFetcher::DataMgrService,
                    expired_at: expired_at_naive,
                };

                let resolve: Resolve = Resolve {
                    uuid: Uuid::new_v4(),
                    source: DataSource::SpaceId,
                    system: DomainNameSystem::Genome,
                    name: genome_domain.clone(),
                    fetcher: DataFetcher::DataMgrService,
                    updated_at: naive_now(),
                };

                let collection_edge = PartOfCollection {
                    platform: Platform::Genome,
                    name: genome_domain.clone(),
                    tld: EXT::Gno.to_string(),
                    status: DomainStatus::Taken,
                };

                if d.is_default {
                    // 'reverse' resolution maps from an address back to a name.
                    let reverse: Resolve = Resolve {
                        uuid: Uuid::new_v4(),
                        source: DataSource::SpaceId,
                        system: DomainNameSystem::Genome,
                        name: genome_domain.clone(),
                        fetcher: DataFetcher::DataMgrService,
                        updated_at: naive_now(),
                    };
                    debug!("{} => Genome({}) is_default", address, genome_domain);
                    let rr = reverse.wrapper(&addr, &gno, REVERSE_RESOLVE);
                    edges.push(EdgeWrapperEnum::new_reverse_resolve(rr));
                }

                // 'regular' resolution involves mapping from a name to an address.
                let rs = resolve.wrapper(&gno, &addr, RESOLVE);
                // hold record
                let hd = hold.wrapper(&addr, &gno, HOLD_IDENTITY);
                // create collection edge
                let c = collection_edge.wrapper(&domain_collection, &gno, PART_OF_COLLECTION);

                edges.push(EdgeWrapperEnum::new_resolve(rs));
                edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
            }
        }

        Ok(edges)
    }
}
