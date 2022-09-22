// Upstreams
mod aggregation;
mod ens_reverse;
mod keybase;
mod knn3;
mod proof_client;
mod rss3;
mod sybil_list;
#[cfg(test)]
mod tests;
mod the_graph;
mod types;

use std::{
    collections::HashSet,
    ops::DerefMut,
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    error::Error,
    upstream::{
        aggregation::Aggregation, ens_reverse::ENSReverseLookup, keybase::Keybase, knn3::Knn3,
        proof_client::ProofClient, rss3::Rss3, sybil_list::SybilList, the_graph::TheGraph,
    },
    util::{hashset_append, hashset_exists, hashset_pop, hashset_push},
};
use async_trait::async_trait;
use futures::future::join_all;
use log::{debug, info, warn};
use tokio::time::{sleep, Duration};
pub(crate) use types::{DataFetcher, DataSource, Platform, Target, TargetProcessedList};

// Maybe we should use Actor model to achieve the same goal here.
// or stream::buffer_unordered ?
lazy_static! {
    /// Global upstream fetching process job queue.
    pub static ref UP_NEXT: Arc<Mutex<HashSet<Target>>> = Arc::new(Mutex::new(HashSet::new()));

    /// Recent processed list.
    pub static ref PROCESSED: Arc<Mutex<HashSet<Target>>> = Arc::new(Mutex::new(HashSet::new()));
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
pub fn fetch_all(initial_target: Target) {
    if hashset_exists(&PROCESSED, &initial_target) {
        return;
    }
    // Later be `pop()`-ed by fetch worker.
    hashset_push(&UP_NEXT, initial_target.clone());
}

/// Find one (platform, identity) pair in all upstreams.
/// Returns identities just fetched for next iter..
pub async fn fetch_one() -> Result<TargetProcessedList, Error> {
    match hashset_pop(&UP_NEXT) {
        Some(ref target) => {
            if !hashset_push(&PROCESSED, target.clone()) { // Already processed.
                return Ok(vec![]);
            };
            let mut up_next: TargetProcessedList = join_all(vec![
                Aggregation::fetch(target),
                SybilList::fetch(target),
                Keybase::fetch(target),
                ProofClient::fetch(target),
                Rss3::fetch(target),
                Knn3::fetch(target),
                TheGraph::fetch(target),
                ENSReverseLookup::fetch(target),
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
            hashset_append(&UP_NEXT, up_next.clone());
            info!("UP_NEXT: added {} items.", up_next.len());

            Ok(up_next)
        }
        None => Ok(vec![]),
    }
}

/// Prefetch all prefetchable upstreams, e.g. SybilList.
pub async fn prefetch() -> Result<(), Error> {
    info!("Prefetching sybil_list ...");
    sybil_list::prefetch().await?;
    info!("Prefetch completed.");
    Ok(())
}

/// Start an upstream fetching worker.
/// NOTE: how about represent worker as a `struct`?
pub fn start_fetch_worker(worker_name: String) {
    info!("Upstream worker {}: started.", worker_name);
    thread::spawn(move || {
        tokio::spawn(async move {
            loop {
                match fetch_one().await {
                    Ok(up_next) => {
                        if up_next.len() == 0 {
                            debug!("Upstream worker {}: nothing fetched.", worker_name);
                            sleep(Duration::from_millis(300)).await;
                        } else {
                            debug!(
                                "Upstream worker {}: {} fetched.",
                                worker_name,
                                up_next.len()
                            );
                        }
                    }
                    Err(err) => {
                        debug!(
                            "Upstream worker {}: error when fetching upstreams: {}",
                            worker_name, err
                        );
                        sleep(Duration::from_millis(300)).await;
                    }
                }
            }
        })
    });
}

/// Start a PROCESSED set cleanse worker.
pub fn start_cleanse_worker() {
    info!("Cleanse worker: started.");
    thread::spawn(move || {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                PROCESSED.clone().lock().unwrap().deref_mut().clear();
                debug!("Cleanse worker: PROCESSED queue cleaned.");
            }
        })
    });
}

/// Start a batch of upstream fetching workers.
pub fn start_fetch_workers(count: usize) {
    for i in 0..count {
        start_fetch_worker(format!("{}", i));
    }
}
