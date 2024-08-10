mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{
    Hold, HyperEdge, PartOfCollection, Resolve, Wrapper, HOLD_IDENTITY, HYPER_EDGE,
    PART_OF_COLLECTION, RESOLVE, REVERSE_RESOLVE,
};
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_identity_hold_record;
use crate::tigergraph::vertex::{DomainCollection, IdentitiesGraph, Identity};
use crate::tigergraph::{EdgeList, EdgeWrapperEnum};
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, DomainSearch, Fetcher, Platform, Target,
    TargetProcessedList, EXT,
};
use crate::util::{make_client, make_http_client, naive_now, parse_body, request_with_timeout};
use async_trait::async_trait;
use futures::future::join_all;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Body, Client, Method};
use serde::Deserialize;
use tracing::{debug, error, warn};
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct BadResponse {
    pub code: String,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct DomainResponse {
    pub meta: Meta,
    pub records: Records,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct ReverseResponse {
    pub meta: Meta,
    pub records: Records,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RecordsForOwnerResponse {
    pub data: Vec<Item>,
    pub meta: MetaList,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetDomainByOwnerResp {
    pub data: Vec<DataItem>,
    pub next: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct DataItem {
    pub meta: Meta,
    pub records: Records,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct MetaList {
    #[serde(rename = "perPage")]
    pub per_page: i64,
    #[serde(rename = "nextStartingAfter")]
    pub next: String,
    #[serde(rename = "sortBy")]
    pub sort_by: String,
    #[serde(rename = "sortDirection")]
    pub sort_direction: String,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: String,
    pub attributes: Attributes,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Attributes {
    pub meta: Meta,
    pub records: Records,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Meta {
    pub domain: String,
    #[serde(rename = "tokenId")]
    pub token_id: Option<String>,
    pub namehash: Option<String>,
    pub blockchain: Option<String>,
    #[serde(rename = "networkId")]
    pub network_id: i64,
    pub owner: Option<String>,
    pub resolver: Option<String>,
    pub registry: Option<String>,
    pub reverse: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Records {
    #[serde(rename = "crypto.BTC.address")]
    pub btc_address: Option<String>,
    #[serde(rename = "crypto.ETH.address")]
    pub eth_address: Option<String>,
}

const UNKNOWN_OWNER: &str = "0x0000000000000000000000000000000000000000";

pub struct UnstoppableDomains {}

#[async_trait]
impl Fetcher for UnstoppableDomains {
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
            Platform::Ethereum => batch_fetch_by_wallet(target).await,
            Platform::UnstoppableDomains => batch_fetch_by_handle(target).await,
            _ => Ok((vec![], vec![])),
        }
    }

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::UnstoppableDomains, Platform::Ethereum])
    }
}

async fn batch_fetch_by_wallet(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let address = target.identity()?.to_lowercase();

    let reverse_record = fetch_reverse(&address).await?;
    let mut reverse = false;
    let mut primary_domain = String::from("");
    if reverse_record.meta.domain != "" {
        reverse = true;
        primary_domain = reverse_record.meta.domain;
    }

    let mut cnt: u32 = 0;
    let mut next: Option<String> = None;
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    while cnt < u32::MAX {
        let result = fetch_domain_by_owner(&address, next).await?;
        cnt += result.data.len() as u32;

        for item in result.data.iter() {
            let mut addr: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Ethereum,
                identity: address.clone(),
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

            let mut ud: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::UnstoppableDomains,
                identity: item.meta.domain.clone(),
                uid: None,
                created_at: None,
                display_name: Some(item.meta.domain.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };
            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::UnstoppableDomains,
                transaction: Some("".to_string()),
                id: item.meta.token_id.clone().unwrap_or("".to_string()),
                created_at: None,
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: None,
            };

            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::UnstoppableDomains,
                system: DomainNameSystem::UnstoppableDomains,
                name: item.meta.domain.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };

            if reverse && item.meta.domain == primary_domain {
                // reverse = true
                // 'reverse' resolution maps from an address back to a name.
                let reverse: Resolve = Resolve {
                    uuid: Uuid::new_v4(),
                    source: DataSource::UnstoppableDomains,
                    system: DomainNameSystem::UnstoppableDomains,
                    name: item.meta.domain.clone(),
                    fetcher: DataFetcher::RelationService,
                    updated_at: naive_now(),
                };
                addr.reverse = Some(true);
                ud.reverse = Some(true);
                let rrs = reverse.wrapper(&addr, &ud, REVERSE_RESOLVE);
                edges.push(EdgeWrapperEnum::new_reverse_resolve(rrs));
            }
            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
            ));
            edges.push(EdgeWrapperEnum::new_hyper_edge(
                HyperEdge {}.wrapper(&hv, &ud, HYPER_EDGE),
            ));

            let hd = hold.wrapper(&addr, &ud, HOLD_IDENTITY);
            let rs = resolve.wrapper(&ud, &addr, RESOLVE);
            edges.push(EdgeWrapperEnum::new_hold_identity(hd));
            edges.push(EdgeWrapperEnum::new_resolve(rs));
        }

        if result.next.is_some() {
            next = result.next;
        } else {
            break;
        }
    }
    Ok((vec![], edges))
}

async fn batch_fetch_by_handle(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error> {
    let name = target.identity()?;
    let result = fetch_owner_by_domain(&name).await?;
    if result.meta.owner.is_none() {
        warn!("UnstoppableDomains target {} | No Result", target);
        return Ok((vec![], vec![]));
    }

    if result.meta.owner.clone().unwrap().to_lowercase() == UNKNOWN_OWNER {
        warn!("UnstoppableDomains owner is zero address {}", target);
        return Ok((vec![], vec![]));
    }

    let mut next_targets = TargetProcessedList::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    let mut addr: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: result.meta.owner.clone().unwrap().to_lowercase(),
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

    let mut ud: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::UnstoppableDomains,
        identity: result.meta.domain.clone(),
        uid: None,
        created_at: None,
        display_name: Some(result.meta.domain.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        transaction: None,
        id: result.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        system: DomainNameSystem::UnstoppableDomains,
        name: result.meta.domain.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    if let Some(reverse) = result.meta.reverse {
        if reverse {
            // reverse = true
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::UnstoppableDomains,
                system: DomainNameSystem::UnstoppableDomains,
                name: result.meta.domain.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };
            addr.reverse = Some(true);
            ud.reverse = Some(true);
            let rrs = reverse.wrapper(&addr, &ud, REVERSE_RESOLVE);
            edges.push(EdgeWrapperEnum::new_reverse_resolve(rrs));
        }
    }

    edges.push(EdgeWrapperEnum::new_hyper_edge(
        HyperEdge {}.wrapper(&hv, &addr, HYPER_EDGE),
    ));
    edges.push(EdgeWrapperEnum::new_hyper_edge(
        HyperEdge {}.wrapper(&hv, &ud, HYPER_EDGE),
    ));

    let hd = hold.wrapper(&addr, &ud, HOLD_IDENTITY);
    let rs = resolve.wrapper(&ud, &addr, RESOLVE);
    edges.push(EdgeWrapperEnum::new_hold_identity(hd));
    edges.push(EdgeWrapperEnum::new_resolve(rs));

    next_targets.push(Target::Identity(
        Platform::Ethereum,
        result.meta.owner.clone().unwrap().to_lowercase(),
    ));

    Ok((next_targets, edges))
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Ethereum => fetch_domains_by_account(platform, identity).await,
        Platform::UnstoppableDomains => fetch_account_by_domain(platform, identity).await,
        _ => Ok(vec![]),
    }
}

async fn fetch_domain_by_owner(
    owners: &str,
    next: Option<String>,
) -> Result<GetDomainByOwnerResp, Error> {
    let client = make_client();
    // curl --request GET "https://api.unstoppabledomains.com/resolve/owners/0x50b6a9ba0b1ca77ce67c22b30afc0a5bbbdb5a18/domains"
    let uri: http::Uri = if next.is_none() {
        format!(
            "{}/resolve/owners/{}/domains",
            C.upstream.unstoppable_api.url, owners
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    } else {
        // "next": "/owners/0x8aad44321a86b170879d7a244c1e8d360c99dda8/domains?cursor=123"
        format!(
            "{}/resolve/{}",
            C.upstream.unstoppable_api.url,
            next.unwrap()
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    };

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut body = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    // Parse response body
    let result = match parse_body::<GetDomainByOwnerResp>(&mut body).await {
        Ok(result) => result,
        Err(_) => {
            match parse_body::<BadResponse>(&mut body).await {
                Ok(bad) => {
                    let err_message = format!(
                        "UnstoppableDomains fetch error, Code: {}, Message: {}",
                        bad.code, bad.message
                    );
                    error!(err_message);
                    return Err(Error::General(
                        err_message,
                        lambda_http::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
                Err(err) => return Err(err),
            };
        }
    };
    Ok(result)
}

async fn fetch_owner_by_domain(domains: &str) -> Result<DomainResponse, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/resolve/domains/{}",
        C.upstream.unstoppable_api.url, domains
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    let result = match parse_body::<DomainResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => match parse_body::<BadResponse>(&mut resp).await {
            Ok(bad) => {
                let err_message = format!(
                    "UnstoppableDomains fetch | errCode: {}, errMessage: {}",
                    bad.code, bad.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
            Err(err) => return Err(err),
        },
    };

    Ok(result)
}

/// Do not use `fetch_domain` query
#[allow(dead_code)]
async fn fetch_domain(owners: &str, page: &str) -> Result<RecordsForOwnerResponse, Error> {
    let client = make_client();
    let uri: http::Uri = if page.is_empty() {
        format!(
            "{}/domains?owners={}",
            C.upstream.unstoppable_api.url, owners
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    } else {
        format!(
            "{}/domains?owners={}&startingAfter={}",
            C.upstream.unstoppable_api.url, owners, page
        )
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?
    };

    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut body = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    // Parse response body
    let result = match parse_body::<RecordsForOwnerResponse>(&mut body).await {
        Ok(result) => result,
        Err(_) => {
            match parse_body::<BadResponse>(&mut body).await {
                Ok(bad) => {
                    let err_message = format!(
                        "UnstoppableDomains fetch error, Code: {}, Message: {}",
                        bad.code, bad.message
                    );
                    error!(err_message);
                    return Err(Error::General(
                        err_message,
                        lambda_http::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
                Err(err) => return Err(err),
            };
        }
    };
    Ok(result)
}

async fn fetch_reverse(owner: &str) -> Result<ReverseResponse, Error> {
    let client = make_client();
    // https://api.unstoppabledomains.com/resolve/reverse/{owner}
    let reverse_uri: http::Uri = format!(
        "{}/resolve/reverse/{}",
        C.upstream.unstoppable_api.url, owner
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let reverse_req = hyper::Request::builder()
        .method(Method::GET)
        .uri(reverse_uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut reverse_resp = request_with_timeout(&client, reverse_req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch reverse record: {:?}",
                err.to_string()
            ))
        })?;

    let result = match parse_body::<ReverseResponse>(&mut reverse_resp).await {
        Ok(r) => r,
        Err(_) => {
            let err: BadResponse = parse_body(&mut reverse_resp).await?;
            let err_message = format!(
                "UnstoppableDomains reverse fetch error, Code: {}, Message: {}",
                err.code, err.message
            );
            error!(err_message);

            return Err(Error::General(err_message, reverse_resp.status()));
        }
    };

    Ok(result)
}

async fn save_domain(
    client: &Client<HttpConnector>,
    identity: &str,
    item: Item,
) -> Result<TargetProcessedList, Error> {
    let mut eth_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: identity.to_string().to_lowercase().clone(),
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

    let mut ud: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::UnstoppableDomains,
        identity: item.id.clone(),
        uid: None,
        created_at: None,
        display_name: Some(item.attributes.meta.domain.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };
    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        transaction: None,
        id: item.attributes.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        system: DomainNameSystem::UnstoppableDomains,
        name: item.id.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    if let Some(reverse) = item.attributes.meta.reverse {
        if reverse {
            // reverse = true
            // 'reverse' resolution maps from an address back to a name.
            let reverse: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::UnstoppableDomains,
                system: DomainNameSystem::UnstoppableDomains,
                name: item.id.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };
            eth_identity.reverse = Some(true);
            ud.reverse = Some(false);
            create_identity_domain_reverse_resolve_record(client, &eth_identity, &ud, &reverse)
                .await?;
            return Ok(vec![Target::Identity(
                Platform::UnstoppableDomains,
                item.attributes.meta.domain.clone(),
            )]);
        }
    }

    // hold record
    create_identity_to_identity_hold_record(client, &eth_identity, &ud, &hold).await?;
    // 'regular' resolution involves mapping from a name to an address.
    create_identity_domain_resolve_record(client, &ud, &eth_identity, &resolve).await?;

    Ok(vec![])
}

async fn fetch_domains_by_account(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let mut cnt: u32 = 0;
    let mut next = String::from("");
    let mut next_targets: Vec<Target> = Vec::new();
    while cnt < u32::MAX {
        let result = fetch_domain(identity, &next).await?;
        let cli = make_http_client();
        cnt += result.data.len() as u32;

        let futures: Vec<_> = result
            .data
            .into_iter()
            .map(|item| save_domain(&cli, identity, item))
            .collect();

        let targets: TargetProcessedList = join_all(futures)
            .await
            .into_iter()
            .flat_map(|result| result.unwrap_or_default())
            .collect();

        next_targets.extend(targets);
        if result.meta.has_more {
            next = result.meta.next;
        } else {
            break;
        }
    }
    Ok(next_targets)
}

async fn fetch_owner(domains: &str) -> Result<DomainResponse, Error> {
    let client = make_client();
    let uri: http::Uri = format!("{}/domains/{}", C.upstream.unstoppable_api.url, domains)
        .parse()
        .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(
            "Authorization",
            format!("Bearer {}", C.upstream.unstoppable_api.token),
        )
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("Invalid Head Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains fetch | Fail to fetch_domain record: {:?}",
                err.to_string()
            ))
        })?;

    let result = match parse_body::<DomainResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => match parse_body::<BadResponse>(&mut resp).await {
            Ok(bad) => {
                let err_message = format!(
                    "UnstoppableDomains fetch | errCode: {}, errMessage: {}",
                    bad.code, bad.message
                );
                error!(err_message);
                return Err(Error::General(err_message, resp.status()));
            }
            Err(err) => return Err(err),
        },
    };

    Ok(result)
}

async fn fetch_account_by_domain(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let result = fetch_owner(identity).await?;
    if result.meta.owner.is_none() {
        return Ok(vec![]);
    }

    if result.meta.owner.clone().unwrap().to_lowercase() == UNKNOWN_OWNER {
        warn!("UnstoppableDomains owner is zero address");
        return Err(Error::NoResult);
    }

    let mut eth: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: result.meta.owner.clone().unwrap().to_lowercase(),
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

    let mut ud: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::UnstoppableDomains,
        identity: result.meta.domain.clone(),
        uid: None,
        created_at: None,
        display_name: Some(result.meta.domain.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        transaction: None,
        id: result.meta.token_id.unwrap_or("".to_string()),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::UnstoppableDomains,
        system: DomainNameSystem::UnstoppableDomains,
        name: result.meta.domain.clone(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    if let Some(reverse) = result.meta.reverse {
        if reverse {
            // reverse = true
            // 'reverse' resolution maps from an address back to a name.
            let resolve: Resolve = Resolve {
                uuid: Uuid::new_v4(),
                source: DataSource::UnstoppableDomains,
                system: DomainNameSystem::UnstoppableDomains,
                name: result.meta.domain.clone(),
                fetcher: DataFetcher::RelationService,
                updated_at: naive_now(),
            };
            eth.reverse = Some(true);
            ud.reverse = Some(true);
            create_identity_domain_reverse_resolve_record(&cli, &eth, &ud, &resolve).await?;
        }
    }

    // hold record
    create_identity_to_identity_hold_record(&cli, &eth, &ud, &hold).await?;
    // 'regular' resolution involves mapping from a name to an address.
    create_identity_domain_resolve_record(&cli, &ud, &eth, &resolve).await?;

    Ok(vec![Target::Identity(
        Platform::Ethereum,
        result.meta.owner.clone().unwrap().to_lowercase(),
    )])
}

#[async_trait]
impl DomainSearch for UnstoppableDomains {
    async fn domain_search(name: &str) -> Result<EdgeList, Error> {
        let mut process_name = name.to_string();
        if name.contains(".") {
            process_name = name.split(".").next().unwrap_or("").to_string();
        }
        if process_name == "".to_string() {
            warn!("UnstoppableDomains domain_search(name='') is not a valid domain name");
            return Ok(vec![]);
        }
        debug!("UnstoppableDomains domain_search(name={})", process_name);

        let result = domain_search(&process_name).await?;

        let mut edges = EdgeList::new();
        let domain_collection = DomainCollection {
            label: process_name.clone(),
            updated_at: naive_now(),
        };

        for r in result.iter() {
            let ud_name = r.domain.name.clone();
            let tld_name = r.domain.extension.clone();
            let tld: EXT = tld_name.parse()?;
            if tld == EXT::Unknown {
                continue;
            }
            if r.availability == false {
                if r.status == "registered".to_string() {
                    let mut ud: Identity = Identity {
                        uuid: Some(Uuid::new_v4()),
                        platform: Platform::UnstoppableDomains,
                        identity: ud_name.clone(),
                        uid: None,
                        created_at: None,
                        display_name: Some(ud_name.clone()),
                        added_at: naive_now(),
                        avatar_url: None,
                        profile_url: None,
                        updated_at: naive_now(),
                        expired_at: None,
                        reverse: Some(false),
                    };
                    let collection_edge = PartOfCollection {
                        system: DomainNameSystem::UnstoppableDomains.to_string(),
                        name: ud_name.clone(),
                        tld: tld.to_string(),
                        status: "taken".to_string(),
                    };

                    let owner_result = fetch_owner_by_domain(&ud_name).await?;
                    if owner_result.meta.owner.is_none() {
                        warn!(
                            "UnstoppableDomains fetch_owner_by_domain({}) | No Result",
                            ud_name
                        );
                        let c =
                            collection_edge.wrapper(&domain_collection, &ud, PART_OF_COLLECTION);
                        edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
                        continue;
                    }

                    if owner_result.meta.owner.clone().unwrap().to_lowercase() == UNKNOWN_OWNER {
                        warn!(
                            "UnstoppableDomains fetch_owner_by_domain({}) owner is zero address",
                            ud_name
                        );
                        let c =
                            collection_edge.wrapper(&domain_collection, &ud, PART_OF_COLLECTION);
                        edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
                        continue;
                    }

                    let mut addr: Identity = Identity {
                        uuid: Some(Uuid::new_v4()),
                        platform: Platform::Ethereum,
                        identity: owner_result.meta.owner.clone().unwrap().to_lowercase(),
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
                        source: DataSource::UnstoppableDomains,
                        transaction: None,
                        id: owner_result.meta.token_id.unwrap_or("".to_string()),
                        created_at: None,
                        updated_at: naive_now(),
                        fetcher: DataFetcher::RelationService,
                        expired_at: None,
                    };

                    let resolve: Resolve = Resolve {
                        uuid: Uuid::new_v4(),
                        source: DataSource::UnstoppableDomains,
                        system: DomainNameSystem::UnstoppableDomains,
                        name: owner_result.meta.domain.clone(),
                        fetcher: DataFetcher::RelationService,
                        updated_at: naive_now(),
                    };

                    if let Some(reverse) = owner_result.meta.reverse {
                        if reverse {
                            // reverse = true
                            // 'reverse' resolution maps from an address back to a name.
                            let reverse: Resolve = Resolve {
                                uuid: Uuid::new_v4(),
                                source: DataSource::UnstoppableDomains,
                                system: DomainNameSystem::UnstoppableDomains,
                                name: owner_result.meta.domain.clone(),
                                fetcher: DataFetcher::RelationService,
                                updated_at: naive_now(),
                            };
                            addr.reverse = Some(true);
                            ud.reverse = Some(true);
                            let rrs = reverse.wrapper(&addr, &ud, REVERSE_RESOLVE);
                            edges.push(EdgeWrapperEnum::new_reverse_resolve(rrs));
                        }
                    }

                    let hd = hold.wrapper(&addr, &ud, HOLD_IDENTITY);
                    let rs = resolve.wrapper(&ud, &addr, RESOLVE);
                    edges.push(EdgeWrapperEnum::new_hold_identity(hd));
                    edges.push(EdgeWrapperEnum::new_resolve(rs));

                    // add domain_collection -> ud_identity
                    let c = collection_edge.wrapper(&domain_collection, &ud, PART_OF_COLLECTION);
                    edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
                } else if r.status == "trademark".to_string() {
                    let ud: Identity = Identity {
                        uuid: Some(Uuid::new_v4()),
                        platform: Platform::UnstoppableDomains,
                        identity: ud_name.clone(),
                        uid: None,
                        created_at: None,
                        display_name: Some(ud_name.clone()),
                        added_at: naive_now(),
                        avatar_url: None,
                        profile_url: None,
                        updated_at: naive_now(),
                        expired_at: None,
                        reverse: Some(false),
                    };
                    let collection_edge = PartOfCollection {
                        system: DomainNameSystem::UnstoppableDomains.to_string(),
                        name: ud_name.clone(),
                        tld: tld.to_string(),
                        status: "protected".to_string(),
                    };

                    let c = collection_edge.wrapper(&domain_collection, &ud, PART_OF_COLLECTION);
                    edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
                }
            }
        }
        Ok(edges)
    }
}

#[derive(Deserialize, Debug, Clone)]
struct SearchResult {
    #[serde(rename = "searchQuery")]
    search_query: String,
    // #[serde(rename = "invalidCharacters")]
    // invalid_characters: Option<Vec<String>>,
    #[serde(rename = "invalidReason")]
    invalid_reason: Option<String>,
    exact: Vec<Exact>,
}

#[derive(Deserialize, Debug, Clone)]
struct Exact {
    status: String,
    availability: bool,
    domain: DomainInfo,
}

#[derive(Deserialize, Debug, Clone)]
struct DomainInfo {
    name: String, // name.ext
    // label: String,     // only name(without extension)
    extension: String, // extension
}

// https://api.unstoppabledomains.com/api/domain/search/internal?q=0xbillys
async fn domain_search(name: &str) -> Result<Vec<Exact>, Error> {
    let client = make_client();
    let encoded_name = urlencoding::encode(name);
    let uri: http::Uri = format!(
        "{}/api/domain/search/internal?q={}",
        C.upstream.unstoppable_api.url, encoded_name,
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;
    let req = hyper::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| {
            Error::ParamError(format!(
                "unstoppabledomains search domain(search_query={}) invalid error {}",
                name, _err
            ))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "UnstoppableDomains search | Fail to search domain(search_query={}): {:?}",
                name,
                err.to_string()
            ))
        })?;
    if !resp.status().is_success() {
        let err_message = format!(
            "UnstoppableDomains search domain http error, statusCode: {}",
            resp.status()
        );
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<SearchResult>(&mut resp).await {
        Ok(result) => {
            if result.invalid_reason.is_some() {
                let err_message = format!(
                    "UnstoppableDomains search domain(search_query={}) invalid_reason : {:?}",
                    result.search_query, result.invalid_reason
                );
                error!(err_message);
                return Err(Error::ManualHttpClientError(err_message));
            }
            result.exact
        }
        Err(err) => {
            let err_message = format!(
                "UnstoppableDomains search domain(search_query={}) error parse_body error: {:?}",
                name, err
            );
            error!(err_message);
            return Err(Error::ManualHttpClientError(err_message));
        }
    };

    Ok(result)
}
