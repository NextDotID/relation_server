// Upstreams
mod aggregation;
mod ens_reverse;
mod keybase;
mod knn3;
mod proof_client;
mod rss3;
mod sybil_list;
mod the_graph;

mod types;
pub(crate) use types::{DataFetcher, DataSource, Platform, Target, TargetProcessedList};

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use crate::{
    error::Error,
    upstream::{
        aggregation::Aggregation, ens_reverse::ENSReverseLookup, keybase::Keybase, knn3::Knn3,
        proof_client::ProofClient, rss3::Rss3, sybil_list::SybilList, the_graph::TheGraph,
    },
    util::{queue_append, queue_pop, queue_push},
};
use async_trait::async_trait;
use futures::future::join_all;
use log::{debug, info, warn};
use tokio::time::{sleep, Duration};

lazy_static! {
    /// Global upstream fetching process job queue.
    pub static ref UP_NEXT: Arc<Mutex<TargetProcessedList>> = Arc::new(Mutex::new(vec![]));

    // TODO: recent processed list.
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
pub async fn fetch_all(initial_target: Target) -> Result<(), Error> {
    queue_push(&UP_NEXT, initial_target);

    // let mut up_next: TargetProcessedList = vec![initial_target];
    // let mut processed: TargetProcessedList = vec![];
    // while !up_next.is_empty() {
    //     debug!("fetch_all::up_next | {:?}", up_next);
    //     let target = up_next.pop().unwrap();
    //     // let fetched = fetch_one(&target).await?;
    //     processed.push(target.clone());
    //     // fetched.into_iter().for_each(|f| {
    //     //     if processed.contains(&f) || up_next.contains(&f) {
    //     //         info!("fetch_all::iter | Fetched {} | duplicated", f);
    //     //     } else {
    //     //         up_next.push(f.clone());
    //     //         info!(
    //     //             "fetch_all::iter | Fetched {} | pushed into up_next",
    //     //             f.clone()
    //     //         );
    //     //     }
    //     // });
    // }
    Ok(())
}

/// Find one (platform, identity) pair in all upstreams.
/// Returns identities just fetched for next iter..
pub async fn fetch_one() -> Result<TargetProcessedList, Error> {
    let target = queue_pop(&UP_NEXT);

    if let Some(ref target) = target {
        info!("Now processing {}", target);
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
        let mut up_next_to_append = up_next.clone();
        queue_append(&UP_NEXT, &mut up_next_to_append);
        info!("UP_NEXT: added {} items.", up_next.len());

        Ok(up_next)
    } else {
        Ok(vec![])
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
/// NOTE: how about reporesent worker as a `struct`?
pub fn start_fetch_worker(worker_name: String) {
    info!("Upstream worker {}: started.", worker_name);
    tokio::spawn(async move {
        loop {
            match fetch_one().await {
                Ok(up_next) => if up_next.len() == 0 {
                    debug!("Upstream worker {}: nothing fetched.", worker_name);
                    sleep(Duration::from_millis(300)).await;
                } else {
                    debug!("Upstream worker {}: {} fetched.", worker_name, up_next.len());
                },
                Err(err) => {
                    debug!("Upstream worker {}: error when fetching upstreams: {}", worker_name, err);
                    sleep(Duration::from_millis(300)).await;
                },
            }
        }
    });
}

/// Start a batch of upstream fetching workers.
pub fn start_fetch_workers(count: usize) {
    for i in 0..count {
        start_fetch_worker(format!("{}", i));
    }
}
