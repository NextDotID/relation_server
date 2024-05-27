// Upstreams
mod aggregation;
mod crossbell;
mod dotbit;
mod ens_reverse;
mod farcaster;
mod genome;
mod keybase;
mod knn3;
mod lensv2;
mod proof_client;
mod rss3;
mod solana;
mod space_id;
mod sybil_list;
mod unstoppable;
// mod firefly;
// mod opensea;

#[cfg(test)]
mod tests;
mod the_graph;
mod types;

use crate::{
    error::Error,
    tigergraph::{batch_upsert, EdgeList},
    upstream::{
        crossbell::Crossbell, dotbit::DotBit, ens_reverse::ENSReverseLookup, farcaster::Farcaster,
        genome::Genome, keybase::Keybase, knn3::Knn3, lensv2::LensV2, proof_client::ProofClient,
        rss3::Rss3, solana::Solana, space_id::SpaceId, sybil_list::SybilList, the_graph::TheGraph,
        unstoppable::UnstoppableDomains,
    },
    util::{hashset_append, make_http_client},
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

    /// Fetch all vertices and edges from given source and return them
    async fn batch_fetch(target: &Target) -> Result<(TargetProcessedList, EdgeList), Error>;

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
    let mut up_next: HashSet<Target> = HashSet::new();
    let mut all_edges: EdgeList = EdgeList::new();

    let mut fetching = FETCHING.lock().await;
    up_next.extend(
        targets
            .clone()
            .into_iter()
            .filter(|target| !fetching.contains(target)),
    );
    up_next.iter().for_each(|target| {
        fetching.insert(target.clone());
    });
    drop(fetching);

    let mut processed: HashSet<Target> = HashSet::new();

    while !up_next.is_empty() {
        round += 1;
        let (next_targets, edges) = fetch_many(
            up_next
                .clone()
                .into_iter()
                .filter(|target| !processed.contains(target))
                .collect(),
            Some(round),
        )
        .await?;

        hashset_append(&mut processed, up_next.into_iter().collect());
        up_next = HashSet::from_iter(next_targets.into_iter());

        all_edges.extend(edges);

        if depth.is_some() && depth.unwrap() <= round {
            // Fork as background job to continue fetching.
            tokio::spawn(fetch_all(up_next.into_iter().collect(), None));
            break;
        }
    }

    let mut fetching = FETCHING.lock().await;
    targets.iter().for_each(|target| {
        fetching.remove(&target);
    });
    drop(fetching);

    // Upsert all edges after fetching completes
    if !all_edges.is_empty() {
        let cli = make_http_client();
        batch_upsert(&cli, all_edges).await?;
    }

    event!(
        Level::INFO,
        round,
        ?depth,
        processed = processed.len(),
        "Fetch completed."
    );

    Ok(())

    // let mut round: u16 = 0;
    // let mut fetching = FETCHING.lock().await;
    // let mut up_next: HashSet<Target> = HashSet::from_iter(
    //     targets
    //         .clone()
    //         .into_iter()
    //         .filter(|target| !fetching.contains(target)),
    // );
    // up_next.iter().for_each(|target| {
    //     fetching.insert(target.clone());
    //     ()
    // });
    // drop(fetching);
    // let mut processed: HashSet<Target> = HashSet::new();

    // // FETCHING.lock().await.insert(initial_target.clone());
    // // queues of this session.
    // // let mut up_next = HashSet::from([initial_target.clone()]);
    // // let mut processed: HashSet<Target> = HashSet::new();

    // while up_next.len() > 0 {
    //     round += 1;
    //     let result = fetch_many(
    //         up_next
    //             .clone()
    //             .into_iter()
    //             .filter(|target| !processed.contains(target))
    //             .collect(),
    //         Some(round),
    //     )
    //     .await?;
    //     if depth.is_some() && depth.unwrap() <= round {
    //         // Fork as background job to continue fetching.
    //         tokio::spawn(fetch_all(up_next.into_iter().collect(), None));
    //         break;
    //     }

    //     // Add previous up_next into processed, and replace with new up_next
    //     hashset_append(&mut processed, up_next.into_iter().collect());
    //     up_next = HashSet::from_iter(result.into_iter());
    // }
    // let mut fetching = FETCHING.lock().await;
    // targets.iter().for_each(|target| {
    //     fetching.remove(&target);
    //     ()
    // });
    // drop(fetching);
    // event!(
    //     Level::INFO,
    //     round,
    //     ?depth,
    //     processed = processed.len(),
    //     "Fetch completed."
    // );
    // Ok(())
}

/// Fetch targets in parallel of 5.
/// `round` is only for log purpose.
pub async fn fetch_many(
    targets: Vec<Target>,
    round: Option<u16>,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    const CONCURRENT: usize = 5;
    let futures: Vec<_> = targets
        .iter()
        .map(|target| batch_fetch_upstream(target))
        .collect();
    let futures_stream = futures::stream::iter(futures).buffer_unordered(CONCURRENT);
    let (mut all_targets, all_edges) = futures_stream
        .fold(
            (TargetProcessedList::new(), EdgeList::new()),
            |(mut all_targets, mut all_edges), handle_result| async move {
                match handle_result {
                    Ok((targets, edges)) => {
                        event!(
                            Level::DEBUG,
                            ?round,
                            fetched_length = targets.len(),
                            "Round completed."
                        );
                        all_targets.extend(targets);
                        all_edges.extend(edges);
                    }
                    Err(err) => {
                        event!(Level::WARN, ?round, %err, "Error happened in fetching task");
                    }
                }
                (all_targets, all_edges)
            },
        )
        .await;
    all_targets.dedup();

    // Instead of upsert edges after each `Round completed`,
    // wait for all data sources to be added after fetch_all ends.
    Ok((all_targets, all_edges))

    // const CONCURRENT: usize = 5;
    // let futures: Vec<_> = targets.iter().map(|target| fetch_one(target)).collect();
    // let futures_stream = futures::stream::iter(futures).buffer_unordered(CONCURRENT);

    // let mut result: TargetProcessedList = futures_stream
    //     .collect::<Vec<Result<Vec<Target>, Error>>>()
    //     .await
    //     .into_iter()
    //     .flat_map(|handle_result| -> Vec<Target> {
    //         match handle_result {
    //             Ok(result) => {
    //                 event!(
    //                     Level::DEBUG,
    //                     ?round,
    //                     fetched_length = result.len(),
    //                     "Round completed."
    //                 );
    //                 result
    //             }
    //             Err(err) => {
    //                 event!(Level::WARN, ?round, %err, "Error happened in fetching task");
    //                 vec![]
    //             }
    //         }
    //     })
    //     .collect();
    // result.dedup();
    // Ok(result)
}

/// Find one (platform, identity) pair in all upstreams.
/// Returns amount of identities just fetched for next iter.
pub async fn fetch_one(target: &Target) -> Result<Vec<Target>, Error> {
    let mut up_next: TargetProcessedList = join_all(vec![
        TheGraph::fetch(target),
        ENSReverseLookup::fetch(target),
        Farcaster::fetch(target),
        LensV2::fetch(target),
        ProofClient::fetch(target),
        Keybase::fetch(target),
        SybilList::fetch(target),
        Rss3::fetch(target),
        Knn3::fetch(target),
        DotBit::fetch(target),
        UnstoppableDomains::fetch(target),
        SpaceId::fetch(target),
        Genome::fetch(target),
        Crossbell::fetch(target),
        Solana::fetch(target),
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

pub async fn batch_fetch_upstream(
    target: &Target,
) -> Result<(TargetProcessedList, EdgeList), Error> {
    let mut up_next = TargetProcessedList::new();
    let mut all_edges = EdgeList::new();

    let _ = join_all(vec![
        TheGraph::batch_fetch(target),
        ENSReverseLookup::batch_fetch(target),
        Farcaster::batch_fetch(target),
        LensV2::batch_fetch(target),
        ProofClient::batch_fetch(target),
        Keybase::batch_fetch(target),
        Rss3::batch_fetch(target),
        DotBit::batch_fetch(target),
        UnstoppableDomains::batch_fetch(target),
        SpaceId::batch_fetch(target),
        Genome::batch_fetch(target),
        Crossbell::batch_fetch(target),
        Solana::batch_fetch(target),
        // SybilList::batch_fetch(target), // move this logic to `data_process` as a scheduled asynchronous fetch
        // Knn3::batch_fetch(target), // Temporarily cancel
        // Firefly::batch_fetch(target), // Temporarily cancel
        // OpenSea::batch_fetch(target), // Temporarily cancel
    ])
    .await
    .into_iter()
    .for_each(|res| {
        if let Ok((next_targets, edges)) = res {
            up_next.extend(next_targets);
            all_edges.extend(edges);
        } else if let Err(err) = res {
            warn!(
                "Error happened when fetching and saving {}: {}",
                target, err
            );
            // Don't break the procedure, continue with other results
        }
    });

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

    // event!(Level::INFO, "fetch_one_and_save up_next {:?}", up_next);
    Ok((up_next, all_edges))
}

/// Prefetch all prefetchable upstreams, e.g. SybilList.
pub async fn prefetch() -> Result<(), Error> {
    info!("Prefetching sybil_list ...");
    sybil_list::prefetch().await?;
    info!("Prefetch completed.");
    Ok(())
}
