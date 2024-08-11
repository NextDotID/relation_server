use crate::{
    config::C,
    error::Error,
    tigergraph::upsert::{create_identity_to_identity_hold_record, create_isolated_vertex},
    tigergraph::{
        EdgeList, EdgeWrapperEnum,
        {
            edge::{
                Hold, HyperEdge, PartOfCollection, Wrapper, HOLD_IDENTITY, HYPER_EDGE,
                PART_OF_COLLECTION,
            },
            vertex::{DomainCollection, IdentitiesGraph, Identity},
        },
    },
    upstream::{DataFetcher, DataSource, Platform, Target, TargetProcessedList, EXT},
    util::{
        make_client, make_http_client, naive_datetime_from_milliseconds,
        naive_datetime_to_milliseconds, naive_now, parse_body, request_with_timeout,
    },
};
use chrono::NaiveDateTime;
use http::uri::InvalidUri;
use hyper::{client::HttpConnector, Client};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};
use uuid::Uuid;

pub async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    match *platform {
        Platform::Farcaster => fetch_by_username(platform, identity).await,
        Platform::Ethereum => fetch_by_signer(platform, identity).await,
        _ => Ok(vec![]),
    }
}

pub async fn batch_fetch_connections(
    platform: &Platform,
    identity: &str,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    match *platform {
        Platform::Farcaster => batch_fetch_by_username(platform, identity).await,
        Platform::Ethereum => batch_fetch_by_signer(platform, identity).await,
        _ => Ok((vec![], vec![])),
    }
}

pub async fn batch_fetch_by_username(
    _platform: &Platform,
    username: &str,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let mut targets: Vec<Target> = Vec::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();
    let user = user_by_username(username).await?;
    if user.is_none() {
        return Ok((vec![], vec![]));
    }
    let user = user.unwrap();
    let fid = user.fid;
    let verifications = get_verifications(fid).await?;
    if verifications.is_none() {
        return Ok((vec![], vec![]));
    }
    let verifications = verifications.unwrap();
    // isolated vertex
    if verifications.is_empty() {
        let isolated_farcaster: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Farcaster,
            identity: user.username.clone(),
            uid: Some(user.fid.to_string()),
            created_at: None,
            display_name: Some(user.display_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };
        edges.push(EdgeWrapperEnum::new_hyper_edge(HyperEdge {}.wrapper(
            &hv,
            &isolated_farcaster,
            HYPER_EDGE,
        )));
        return Ok((vec![], edges));
    }

    for verification in verifications.iter() {
        let protocol: Platform = verification.protocol.parse()?;
        let mut address = verification.address.clone();
        if protocol == Platform::Ethereum {
            address = address.to_lowercase();
        }
        let wallet: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: protocol,
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
        let farcaster: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Farcaster,
            identity: user.username.clone(),
            uid: Some(user.fid.to_string()),
            created_at: None,
            display_name: Some(user.display_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };
        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Farcaster,
            transaction: None,
            id: "".to_string(),
            created_at: Some(verification.timestamp),
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
            expired_at: None,
        };

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &wallet, HYPER_EDGE),
        ));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &farcaster, HYPER_EDGE),
        ));
        let hd = hold.wrapper(&wallet, &farcaster, HOLD_IDENTITY);
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));
        targets.push(Target::Identity(protocol, address.clone()))
    }

    Ok((targets, edges))
}

pub async fn batch_fetch_by_signer(
    platform: &Platform,
    address: &str,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    if platform.to_owned() == Platform::Solana {
        // WrapcastV2 not supported user-by-verification?address=solana_address format yet.
        return Ok((vec![], vec![]));
    }

    let user = user_by_verification(address).await?;
    if user.is_none() {
        return Ok((vec![], vec![]));
    }
    let user = user.unwrap();

    let mut targets: Vec<Target> = Vec::new();
    let mut edges = EdgeList::new();
    let hv = IdentitiesGraph::default();

    let fid = user.fid;
    let verifications = get_verifications(fid).await?;
    if verifications.is_none() {
        return Ok((vec![], vec![]));
    }
    let verifications = verifications.unwrap();
    if verifications.is_empty() {
        return Ok((vec![], vec![]));
    }

    for verification in verifications.iter() {
        let protocol: Platform = verification.protocol.parse()?;
        let mut verification_address = verification.address.clone();
        if protocol == Platform::Ethereum {
            verification_address = verification_address.to_lowercase();
        }
        let wallet: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: protocol,
            identity: verification_address.clone(),
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
        let farcaster: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Farcaster,
            identity: user.username.clone(),
            uid: Some(user.fid.to_string()),
            created_at: None,
            display_name: Some(user.display_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };
        let hold: Hold = Hold {
            uuid: Uuid::new_v4(),
            source: DataSource::Farcaster,
            transaction: None,
            id: "".to_string(),
            created_at: Some(verification.timestamp),
            updated_at: naive_now(),
            fetcher: DataFetcher::RelationService,
            expired_at: None,
        };

        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &wallet, HYPER_EDGE),
        ));
        edges.push(EdgeWrapperEnum::new_hyper_edge(
            HyperEdge {}.wrapper(&hv, &farcaster, HYPER_EDGE),
        ));
        let hd = hold.wrapper(&wallet, &farcaster, HOLD_IDENTITY);
        edges.push(EdgeWrapperEnum::new_hold_identity(hd));

        if address != verification_address {
            // Do not push the same target repeatedly
            targets.push(Target::Identity(protocol, verification_address.clone()))
        }
    }

    targets.push(Target::Identity(Platform::Farcaster, user.username.clone()));
    Ok((targets, edges))
}

pub async fn fetch_by_username(
    _platform: &Platform,
    username: &str,
) -> Result<TargetProcessedList, Error> {
    let cli = make_http_client();
    let user = user_by_username(username).await?;
    if user.is_none() {
        return Ok(vec![]);
    }
    let user = user.unwrap();
    let fid = user.fid;
    let verifications = get_verifications(fid).await?;
    if verifications.is_none() {
        return Ok(vec![]);
    }
    let verifications = verifications.unwrap();
    // isolated vertex
    if verifications.is_empty() {
        let u: Identity = Identity {
            uuid: Some(Uuid::new_v4()),
            platform: Platform::Farcaster,
            identity: user.username.clone(),
            uid: Some(user.fid.to_string()),
            created_at: None,
            display_name: Some(user.display_name.clone()),
            added_at: naive_now(),
            avatar_url: None,
            profile_url: None,
            updated_at: naive_now(),
            expired_at: None,
            reverse: Some(false),
        };
        // let vertices = Vertices(vec![u]);
        create_isolated_vertex(&cli, &u).await?;
    }
    let mut targets: Vec<Target> = Vec::new();
    for verification in verifications.iter() {
        let target = save_verifications(&cli, &user, verification).await?;
        targets.push(target);
    }
    Ok(targets)
}

pub async fn fetch_by_signer(
    platform: &Platform,
    address: &str,
) -> Result<TargetProcessedList, Error> {
    if platform.to_owned() == Platform::Solana {
        // Wrapcast not supported user-by-verification?address=solana format
        return Ok(vec![]);
    }
    let cli = make_http_client();
    let user = user_by_verification(address).await?;
    if user.is_none() {
        return Ok(vec![]);
    }
    let mut targets: Vec<Target> = Vec::new();
    let user = user.unwrap();
    let fid = user.fid;
    let verifications = get_verifications(fid).await?;
    if verifications.is_none() {
        return Ok(vec![]);
    }
    let verifications = verifications.unwrap();
    for verification in verifications.iter() {
        let target = save_verifications(&cli, &user, verification).await?;
        targets.push(target);
    }
    targets.push(Target::Identity(Platform::Farcaster, user.username.clone()));
    Ok(targets)
}

async fn save_verifications(
    client: &Client<HttpConnector>,
    user: &User,
    verification: &Verification,
) -> Result<Target, Error> {
    let protocol: Platform = verification.protocol.parse()?;
    let mut address = verification.address.clone();
    if protocol == Platform::Ethereum {
        address = address.to_lowercase();
    }
    let eth_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: protocol,
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
    let farcaster_identity: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: Platform::Farcaster,
        identity: user.username.clone(),
        uid: Some(user.fid.to_string()),
        created_at: None,
        display_name: Some(user.display_name.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
        expired_at: None,
        reverse: Some(false),
    };
    let hold: Hold = Hold {
        uuid: Uuid::new_v4(),
        source: DataSource::Farcaster,
        transaction: None,
        id: "".to_string(),
        created_at: Some(verification.timestamp),
        updated_at: naive_now(),
        fetcher: DataFetcher::RelationService,
        expired_at: None,
    };
    create_identity_to_identity_hold_record(client, &eth_identity, &farcaster_identity, &hold)
        .await?;
    Ok(Target::Identity(protocol, address.clone()))
}

// {"errors":[{"message":"No FID associated with username checkyou"}]}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserProfileResponse {
    pub errors: Option<Vec<Message>>,
    pub result: Option<UserProfileResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserProfileResult {
    pub user: User,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub fid: i64,
    pub username: String,
    #[serde(rename = "displayName")]
    pub display_name: String,

    pub pfp: Pfp,
    pub profile: Profile,
    #[serde(rename = "followerCount")]
    pub follower_count: i64,
    #[serde(rename = "followingCount")]
    pub following_count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pfp {
    pub url: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub bio: Bio,
    pub location: Location,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bio {
    pub text: String,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Location {
    #[serde(rename = "placeId")]
    pub place_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerificationResponse {
    pub errors: Option<Vec<Message>>,
    pub result: Option<VerificationResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerificationResult {
    pub verifications: Vec<Verification>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Verification {
    pub fid: i64,
    pub address: String,
    #[serde(deserialize_with = "naive_datetime_from_milliseconds")]
    #[serde(serialize_with = "naive_datetime_to_milliseconds")]
    pub timestamp: NaiveDateTime,
    pub protocol: String,
}

async fn user_by_username(username: &str) -> Result<Option<User>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/v2/user-by-username?username={}",
        C.upstream.warpcast_api.url, username
    )
    .parse()
    .map_err(|err: InvalidUri| {
        Error::ParamError(format!(
            "v2/user-by-username?username={} Uri format Error: {}",
            username, err
        ))
    })?;

    let req = hyper::Request::builder()
        .method(http::Method::GET)
        .uri(uri)
        .header(
            "authorization",
            format!("Bearer {}", C.upstream.warpcast_api.token),
        )
        .body(hyper::Body::empty())
        .map_err(|err| {
            Error::ParamError(format!(
                "v2/user-by-username?username={} Request build error: {}",
                username, err
            ))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Warpcast fetch error | failed to fetch user-by-username?username={} | {:?}",
                username,
                err.to_string()
            ))
        })?;

    let result = match parse_body::<UserProfileResponse>(&mut resp).await {
        Ok(r) => match r.errors {
            Some(errors) => {
                let err_message = format!(
                    "Warpcast fetch error| failed to fetch user-by-username?username={}, message: {:?}",
                    username, errors
                );
                error!(err_message);
                None
            }
            None => match r.result {
                None => None,
                Some(res) => Some(res.user),
            },
        },
        Err(err) => {
            return Err(Error::ManualHttpClientError(format!(
                "Warpcast fetch error | parse_body error: {}",
                err
            )));
        }
    };

    Ok(result)
}

async fn user_by_verification(address: &str) -> Result<Option<User>, Error> {
    // ^0[xX][0-9a-fA-F]{40}$
    let pattern = r"^0[xX][0-9a-fA-F]{40}$";
    let re = Regex::new(pattern)
        .map_err(|err| Error::ParamError(format!("Regex pattern error: {}", err)))?;
    if !re.is_match(address) {
        // If the address does not match the pattern, return an error
        // return Err(Error::ParamError("Address must match pattern".into()));
        let err_message = format!("Wrapcaster user-by-verification: address must match pattern");
        error!(err_message);
        return Ok(None);
    }

    let client = make_client();
    let uri: http::Uri = format!(
        "{}/v2/user-by-verification?address={}",
        C.upstream.warpcast_api.url, address
    )
    .parse()
    .map_err(|err: InvalidUri| {
        Error::ParamError(format!(
            "v2/user-by-verification?address={} Uri format Error: {}",
            address, err
        ))
    })?;

    let req = hyper::Request::builder()
        .method(http::Method::GET)
        .uri(uri)
        .header(
            "authorization",
            format!("Bearer {}", C.upstream.warpcast_api.token),
        )
        .body(hyper::Body::empty())
        .map_err(|err| {
            Error::ParamError(format!(
                "v2/user-by-verification?address={} Request build error: {}",
                address, err
            ))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Warpcast fetch error | failed to fetch user-by-verification?address={} | {:?}",
                address,
                err.to_string()
            ))
        })?;

    let result = match parse_body::<UserProfileResponse>(&mut resp).await {
        Ok(r) => match r.errors {
            Some(errors) => {
                let err_message = format!(
                    "Warpcast fetch error| failed to fetch user-by-verification?address={}, message: {:?}",
                    address, errors
                );
                error!(err_message);
                None
            }
            None => match r.result {
                None => None,
                Some(res) => Some(res.user),
            },
        },
        Err(err) => {
            return Err(Error::ManualHttpClientError(format!(
                "Warpcast fetch error | parse_body error: {}",
                err
            )));
        }
    };
    Ok(result)
}

async fn get_verifications(fid: i64) -> Result<Option<Vec<Verification>>, Error> {
    let client = make_client();
    let uri: http::Uri = format!(
        "{}/v2/verifications?fid={}",
        C.upstream.warpcast_api.url, fid
    )
    .parse()
    .map_err(|err: InvalidUri| {
        Error::ParamError(format!(
            "v2/verifications?fid={} Uri format Error: {}",
            fid, err
        ))
    })?;

    let req = hyper::Request::builder()
        .method(http::Method::GET)
        .uri(uri)
        .header(
            "authorization",
            format!("Bearer {}", C.upstream.warpcast_api.token),
        )
        .body(hyper::Body::empty())
        .map_err(|err| {
            Error::ParamError(format!(
                "v2/verifications?fid={} Request build error: {}",
                fid, err
            ))
        })?;

    let mut resp = request_with_timeout(&client, req, None)
        .await
        .map_err(|err| {
            Error::ManualHttpClientError(format!(
                "Warpcast fetch error | failed to fetch verifications?fid={} | {:?}",
                fid,
                err.to_string()
            ))
        })?;

    let result = match parse_body::<VerificationResponse>(&mut resp).await {
        Ok(r) => match r.errors {
            Some(errors) => {
                let err_message = format!(
                    "Warpcast fetch error| failed to fetch verifications?fid={}, message: {:?}",
                    fid, errors
                );
                error!(err_message);
                None
            }
            None => match r.result {
                None => None,
                Some(res) => Some(res.verifications),
            },
        },
        Err(err) => {
            return Err(Error::ManualHttpClientError(format!(
                "Warpcast fetch error | parse_body error: {}",
                err
            )));
        }
    };
    Ok(result)
}

pub async fn domain_search(name: &str) -> Result<EdgeList, Error> {
    if name == "".to_string() {
        warn!("Warpcast user_by_username(name='') is not a valid domain name");
        return Ok(vec![]);
    }

    let check_names = vec![name.to_string(), format!("{}.{}", name, EXT::Eth)];
    let mut edges = EdgeList::new();
    let domain_collection = DomainCollection {
        id: name.to_string(),
        updated_at: naive_now(),
    };

    for username in check_names.into_iter() {
        debug!("Warpcast user_by_username(name={})", username);
        let user = user_by_username(&username).await?;
        if user.is_none() {
            return Ok(vec![]);
        }
        let user = user.unwrap();
        let fid = user.fid;
        let verifications = get_verifications(fid).await?;
        if verifications.is_none() {
            return Ok(vec![]);
        }
        let fname = user.username.clone();
        let fname_tld = match fname.ends_with(&EXT::Eth.to_string()) {
            true => EXT::Eth.to_string(),
            false => "".to_string(),
        };
        let verifications = verifications.unwrap();

        // isolated vertex
        if verifications.is_empty() {
            let isolated_farcaster: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Farcaster,
                identity: fname.clone(),
                uid: Some(fid.to_string()),
                created_at: None,
                display_name: Some(user.display_name.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };
            let collection_edge = PartOfCollection {
                platform: Platform::Farcaster,
                name: fname.clone(),
                tld: fname_tld.clone(),
                status: "taken".to_string(),
            };
            // create collection edge
            let c = collection_edge.wrapper(
                &domain_collection,
                &isolated_farcaster,
                PART_OF_COLLECTION,
            );
            edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
            continue;
        }

        for verification in verifications.iter() {
            let protocol: Platform = verification.protocol.parse()?;
            let mut address = verification.address.clone();
            if protocol == Platform::Ethereum {
                address = address.to_lowercase();
            }
            let wallet: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: protocol,
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
            let farcaster: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Farcaster,
                identity: fname.clone(),
                uid: Some(fid.to_string()),
                created_at: None,
                display_name: Some(user.display_name.clone()),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
                expired_at: None,
                reverse: Some(false),
            };
            let hold: Hold = Hold {
                uuid: Uuid::new_v4(),
                source: DataSource::Farcaster,
                transaction: None,
                id: "".to_string(),
                created_at: Some(verification.timestamp),
                updated_at: naive_now(),
                fetcher: DataFetcher::RelationService,
                expired_at: None,
            };

            let collection_edge = PartOfCollection {
                platform: Platform::Farcaster,
                name: fname.clone(),
                tld: fname_tld.clone(),
                status: "taken".to_string(),
            };

            // hold record
            let hd = hold.wrapper(&wallet, &farcaster, HOLD_IDENTITY);

            // create collection edge
            let c = collection_edge.wrapper(&domain_collection, &farcaster, PART_OF_COLLECTION);
            edges.push(EdgeWrapperEnum::new_hold_identity(hd));
            edges.push(EdgeWrapperEnum::new_domain_collection_edge(c));
        }
    }

    Ok(edges)
}
