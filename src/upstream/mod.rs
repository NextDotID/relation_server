// Upstreams
mod aggregation;
mod crossbell;
mod dotbit;
mod ens_reverse;
mod farcaster;
mod keybase;
mod knn3;
mod lens;
mod proof_client;
mod rss3;
mod space_id;
mod sybil_list;
mod unstoppable;

#[cfg(test)]
mod tests;
mod the_graph;
mod types;

use crate::{
    error::Error,
    upstream::{
        aggregation::Aggregation, dotbit::DotBit, ens_reverse::ENSReverseLookup,
        farcaster::Farcaster, keybase::Keybase, knn3::Knn3, lens::Lens, proof_client::ProofClient,
        rss3::Rss3, space_id::SpaceId, sybil_list::SybilList, the_graph::TheGraph,
        unstoppable::UnstoppableDomains,
    },
    util::hashset_append,
};
use async_trait::async_trait;
use futures::{future::join_all, StreamExt};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;
use tracing::{event, info, warn, Level};

pub(crate) use types::vec_string_to_vec_datasource;
pub(crate) use types::{
    Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, Platform, ProofLevel,
    Target, TargetProcessedList,
};

lazy_static! {
    /// Global processing queue to prevent duplicated query. i.e. multiple same request from frontend.
    pub static ref FETCHING: Arc<Mutex<HashSet<Target>>> = Arc::new(Mutex::new(HashSet::new()));
}

/// Fetcher defines how to fetch data from upstream.
#[async_trait]
pub trait Fetcher {
    /// Fetch data from given source.
    async fn fetch(target: &Target) -> Result<TargetProcessedList, Error>;

    /// Determine if this upstream can fetch this target.
    fn can_fetch(target: &Target) -> bool;
}

/// Find all available (platform, identity) in all `Upstream`s.
/// `depth` controls how many fetch layers should `fetch_all` blocks.
/// The rest `up_next` will be fetched asynchronously.  `None` means
/// fetch till exhausted.
// #[tracing::instrument(name = "fetch_all", level = "trace")]
#[async_recursion::async_recursion]
pub async fn fetch_all(targets: TargetProcessedList, depth: Option<u16>) -> Result<(), Error> {
    let mut round: u16 = 0;

    let mut fetching = FETCHING.lock().await;
    let mut up_next: HashSet<Target> = HashSet::from_iter(
        targets
            .clone()
            .into_iter()
            .filter(|target| !fetching.contains(target)),
    );
    up_next.iter().for_each(|target| {
        fetching.insert(target.clone());
        ()
    });
    drop(fetching);
    let mut processed: HashSet<Target> = HashSet::new();

    // FETCHING.lock().await.insert(initial_target.clone());
    // queues of this session.
    // let mut up_next = HashSet::from([initial_target.clone()]);
    // let mut processed: HashSet<Target> = HashSet::new();

    while up_next.len() > 0 {
        round += 1;
        let result = fetch_many(
            up_next
                .clone()
                .into_iter()
                .filter(|target| !processed.contains(target))
                .collect(),
            Some(round),
        )
        .await?;
        if depth.is_some() && depth.unwrap() <= round {
            // Fork as background job to continue fetching.
            tokio::spawn(fetch_all(up_next.into_iter().collect(), None));
            break;
        }

        // Add previous up_next into processed, and replace with new up_next
        hashset_append(&mut processed, up_next.into_iter().collect());
        up_next = HashSet::from_iter(result.into_iter());
    }

    let mut fetching = FETCHING.lock().await;
    targets.iter().for_each(|target| {
        fetching.remove(&target);
        ()
    });
    drop(fetching);

    event!(
        Level::INFO,
        round,
        ?depth,
        processed = processed.len(),
        "Fetch completed."
    );
    Ok(())
}

/// Fetch targets in parallel of 5.
/// `round` is only for log purpose.
pub async fn fetch_many(targets: Vec<Target>, round: Option<u16>) -> Result<Vec<Target>, Error> {
    const CONCURRENT: usize = 5;
    let futures: Vec<_> = targets.iter().map(|target| fetch_one(target)).collect();
    let futures_stream = futures::stream::iter(futures).buffer_unordered(CONCURRENT);

    let mut result: TargetProcessedList = futures_stream
        .collect::<Vec<Result<Vec<Target>, Error>>>()
        .await
        .into_iter()
        .flat_map(|handle_result| -> Vec<Target> {
            match handle_result {
                Ok(result) => {
                    event!(
                        Level::DEBUG,
                        ?round,
                        fetched_length = result.len(),
                        "Round completed."
                    );
                    result
                }
                Err(err) => {
                    event!(Level::WARN, ?round, %err, "Error happened in fetching task");
                    vec![]
                }
            }
        })
        .collect();
    result.dedup();
    Ok(result)
}
/// Find one (platform, identity) pair in all upstreams.
/// Returns amount of identities just fetched for next iter.
pub async fn fetch_one(target: &Target) -> Result<Vec<Target>, Error> {
    let mut up_next: TargetProcessedList = join_all(vec![
        Aggregation::fetch(target),
        SybilList::fetch(target),
        Keybase::fetch(target),
        ProofClient::fetch(target),
        Rss3::fetch(target),
        Knn3::fetch(target),
        TheGraph::fetch(target),
        ENSReverseLookup::fetch(target),
        DotBit::fetch(target),
        UnstoppableDomains::fetch(target),
        Farcaster::fetch(target),
        SpaceId::fetch(target),
        Lens::fetch(target),
    ])
    .await
    .into_iter()
    .flat_map(|res| {
        match res {
            Ok(up_next_list) => up_next_list,
            Err(err) => {
                warn!("Error happened when fetching {}: {}", target, err);
                vec![] // Don't break the procedure
            }
        }
    })
    .collect();
    up_next.dedup();
    // Filter zero address
    up_next = up_next
        .into_iter()
        .filter(|target| match target {
            Target::Identity(Platform::Ethereum, address) => {
                // Filter zero address (without last 4 digits)
                return !address.starts_with("0x000000000000000000000000000000000000");
            }
            Target::Identity(_, _) => true,
            Target::NFT(_, _, _, _) => true,
        })
        .collect();

    Ok(up_next)
}

/// Prefetch all prefetchable upstreams, e.g. SybilList.
pub async fn prefetch() -> Result<(), Error> {
    info!("Prefetching sybil_list ...");
    sybil_list::prefetch().await?;
    info!("Prefetch completed.");
    Ok(())
}
