mod tests;

use crate::config::C;
use crate::error::Error;
use crate::tigergraph::edge::{Hold, Resolve};
use crate::tigergraph::upsert::create_identity_domain_resolve_record;
use crate::tigergraph::upsert::create_identity_domain_reverse_resolve_record;
use crate::tigergraph::upsert::create_identity_to_identity_hold_record;
use crate::tigergraph::vertex::Identity;
use crate::upstream::{
    DataFetcher, DataSource, DomainNameSystem, Fetcher, Platform, Target, TargetProcessedList,
};
use crate::util::{make_client, make_http_client, naive_now, parse_body, request_with_timeout};
use async_trait::async_trait;
use http::uri::InvalidUri;
use hyper::{Body, Method, Request};
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct BadResponse {
    pub code: i32,
    pub msg: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResolveResponse {
    pub code: i32,
    pub address: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReverseResolveResponse {
    pub code: i32,
    pub name: Option<String>,
}

pub struct SpaceId {}

#[async_trait]
impl Fetcher for SpaceId {
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

    fn can_fetch(target: &Target) -> bool {
        target.in_platform_supported(vec![Platform::SpaceId, Platform::Ethereum])
    }
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Ethereum => fetch_domain_by_address(platform, identity).await,
        Platform::SpaceId => fetch_address_by_domain(platform, identity).await,
        _ => Ok(vec![]),
    }
}
async fn fetch_domain_by_address(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let name = get_name(identity).await?;
    if name.is_none() {
        // name=null, address does not have a valid primary name
        return Ok(vec![]);
    }

    let eth: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: identity.to_lowercase(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(true),
    };

    let sid: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::SpaceId,
        identity: name.clone().unwrap(),
        uid: None,
        created_at: None,
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(true),
    };

    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::SpaceId,
        transaction: None,
        id: "".to_string(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };

    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::SpaceId,
        system: DomainNameSystem::SpaceId,
        name: name.clone().unwrap(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };
    let reverse: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::SpaceId,
        system: DomainNameSystem::SpaceId,
        name: name.clone().unwrap(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // hold record
    create_identity_to_identity_hold_record(&cli, &eth, &sid, &hold).await?;
    // 'regular' resolution involves mapping from a name to an address.
    create_identity_domain_resolve_record(&cli, &sid, &eth, &resolve).await?;
    // 'reverse' resolution maps from an address back to a name.
    create_identity_domain_reverse_resolve_record(&cli, &eth, &sid, &reverse).await?;

    return Ok(vec![Target::Identity(
        Platform::SpaceId,
        name.clone().unwrap(),
    )]);
}

async fn fetch_address_by_domain(
    _platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let address = get_address(identity).await?;

    let mut eth: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Ethereum,
        identity: address.clone().to_lowercase(),
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

    let mut sid: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::SpaceId,
        identity: identity.to_string(),
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
        transaction: None,
        id: "".to_string(),
        created_at: None,
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };
    let resolve: Resolve = Resolve {
        uuid: Uuid::new_v4(),
        source: DataSource::SpaceId,
        system: DomainNameSystem::SpaceId,
        name: identity.to_string(),
        fetcher: DataFetcher::RelationService,
        updated_at: naive_now(),
    };

    // hold record
    create_identity_to_identity_hold_record(&cli, &eth, &sid, &hold).await?;
    // 'regular' resolution involves mapping from a name to an address.
    create_identity_domain_resolve_record(&cli, &sid, &eth, &resolve).await?;

    // lookup reverse resolve name
    if let Some(domain) = get_name(&address).await? {
        // 'reverse' resolution maps from an address back to a name.
        let reverse: Resolve = Resolve {
            uuid: Uuid::new_v4(),
            source: DataSource::SpaceId,
            system: DomainNameSystem::SpaceId,
            name: domain,
            fetcher: DataFetcher::RelationService,
            updated_at: naive_now(),
        };
        eth.reverse = Some(true);
        sid.reverse = Some(true);
        create_identity_domain_reverse_resolve_record(&cli, &eth, &sid, &reverse).await?;
    }

    return Ok(vec![Target::Identity(
        Platform::Ethereum,
        address.clone().to_lowercase(),
    )]);
}

/// Resolve Names: https://docs.space.id/developer-guide/web3-name-sdk/sid-api#resolve-names
async fn get_address(domain: &str) -> Result<String, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/v1/getAddress?tld=bnb&domain={}",
        C.upstream.spaceid_api.url.clone(),
        domain
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("SpaceId Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("SpaceId fetch | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("SpaceId fetch error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }
    let result = match parse_body::<ResolveResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut resp).await?;
            // code=1, domain name is invalid
            // code=1, rpc error
            let err_message = format!(
                "SpaceId fetch error, Code: {}, Message: {}",
                err.code, err.msg
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    if result.address == "0x0000000000000000000000000000000000000000" {
        // domain is valid but has not been registered.
        warn!(
            "SpaceId {} domain is valid but has not been registered.",
            domain
        );
        return Err(Error::NoResult);
    }
    Ok(result.address)
}

/// Reverse Resolve Names: https://docs.space.id/developer-guide/web3-name-sdk/sid-api#reverse-resolve-names
async fn get_name(address: &str) -> Result<Option<String>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/v1/getName?tld=bnb&address={}",
        C.upstream.spaceid_api.url.clone(),
        address
    )
    .parse()
    .map_err(|_err: InvalidUri| Error::ParamError(format!("Uri format Error {}", _err)))?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .map_err(|_err| Error::ParamError(format!("SpaceId Build Request Error {}", _err)))?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!("SpaceId fetch | error: {:?}", err.to_string()))
        })?;

    if !resp.status().is_success() {
        let err_message = format!("SpaceId fetch error, statusCode: {}", resp.status());
        error!(err_message);
        return Err(Error::General(err_message, resp.status()));
    }

    let result = match parse_body::<ReverseResolveResponse>(&mut resp).await {
        Ok(result) => result,
        Err(_) => {
            let err: BadResponse = parse_body(&mut resp).await?;
            // code=1, rpc error
            let err_message = format!(
                "SpaceId fetch error, Code: {}, Message: {}",
                err.code, err.msg
            );
            error!(err_message);
            return Err(Error::General(err_message, resp.status()));
        }
    };
    Ok(result.name)
}
