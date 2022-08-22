#[cfg(test)]
mod tests;

use super::{DataFetcher, Target};
use crate::config::C;
use crate::error::Error;
use crate::graph::edge::Proof;
use crate::graph::vertex::Identity;
use crate::graph::{create_identity_to_identity_record, new_db_connection};
use crate::upstream::{DataSource, Fetcher, Platform, TargetProcessedList};
use crate::util::{make_client, naive_now, parse_body, timestamp_to_naive};
use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub current: u32,
    pub next: u32,
}

#[derive(Deserialize, Debug)]
pub struct Record {
    pub id: String,
    pub sns_handle: String,
    pub sns_platform: String,
    pub web3_addr: String,
    pub web3_platform: String,
    pub source: String,
    pub ens: Option<String>,
    pub create_timestamp: String,
    pub modify_timestamp: String,
}

#[derive(Deserialize, Debug)]
pub struct Response {
    pub pagination: Pagination,
    pub records: Vec<Record>,
}
pub struct Aggregation {}

#[async_trait]
impl Fetcher for Aggregation {
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
        target.in_platform_supported(vec![Platform::Ethereum, Platform::Twitter])
    }
}

async fn fetch_connections_by_platform_identity(
    platform: &Platform,
    identity: &str,
) -> Result<TargetProcessedList, Error> {
    let client = make_client();
    let mut page = 1;

    let mut next_targets: TargetProcessedList = Vec::new();

    loop {
        let uri: http::Uri = match format!(
            "{}?platform={}&identity={}&page={}&size=100",
            C.upstream.aggregation_service.url, platform, identity, page
        )
        .parse()
        {
            Ok(n) => n,
            Err(err) => return Err(Error::ParamError(format!("Uri format Error: {}", err))),
        };

        let mut resp = client.get(uri).await?;
        if !resp.status().is_success() {
            break;
        }

        let body: Response = parse_body(&mut resp).await?;
        if body.records.is_empty() {
            break;
        }

        let futures: Vec<_> = body.records.into_iter().map(save_item).collect();
        let targets: TargetProcessedList = join_all(futures)
            .await
            .into_iter()
            .flat_map(|result| result.unwrap_or_default())
            .collect();
        next_targets.extend(targets);

        if body.pagination.current == body.pagination.next {
            break;
        }
        page = body.pagination.next;
    }

    Ok(next_targets)
}

async fn save_item(p: Record) -> Result<TargetProcessedList, Error> {
    let db = new_db_connection().await?;
    let mut targets = Vec::new();

    let from_platform = Platform::from_str(p.sns_platform.as_str()).unwrap_or(Platform::Unknown);
    let from: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: from_platform,
        identity: p.sns_handle.clone().to_lowercase(),
        created_at: None,
        display_name: Some(p.sns_handle.clone()),
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let to_platform = Platform::from_str(p.web3_platform.as_str()).unwrap_or_default();
    let web3_addr = p.web3_addr.to_lowercase();
    let to: Identity = Identity {
        uuid: Some(Uuid::new_v4()),
        platform: to_platform,
        identity: web3_addr.clone().to_lowercase(),
        created_at: None,
        // Don't use ETH's wallet as display_name, use ENS reversed lookup instead.
        display_name: None,
        added_at: naive_now(),
        avatar_url: None,
        profile_url: None,
        updated_at: naive_now(),
    };

    let create_ms_time: u32 = (p.create_timestamp.parse::<i64>().unwrap() % 1000)
        .try_into()
        .unwrap();
    let update_ms_time: u32 = (p.modify_timestamp.parse::<i64>().unwrap() % 1000)
        .try_into()
        .unwrap();

    let pf: Proof = Proof {
        uuid: Uuid::new_v4(),
        source: DataSource::from_str(p.source.as_str()).unwrap_or(DataSource::Unknown),
        record_id: Some(p.id.clone()),
        created_at: Some(timestamp_to_naive(
            p.create_timestamp.parse::<i64>().unwrap() / 1000,
            create_ms_time,
        )),
        updated_at: timestamp_to_naive(
            p.modify_timestamp.parse::<i64>().unwrap() / 1000,
            update_ms_time,
        ),
        fetcher: DataFetcher::AggregationService,
    };

    let _ = create_identity_to_identity_record(&db, &from, &to, &pf).await;

    targets.push(Target::Identity(to_platform, web3_addr.clone()));
    Ok(targets)
}
