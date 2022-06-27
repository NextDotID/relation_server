mod tests;
use crate::upstream::{Fetcher, Platform, Connection};
use crate::upstream::keybase::Keybase;
use crate::upstream::sybil_list::SybilList;
use crate::upstream::proof_client::ProofClient;
use crate::upstream::aggregation::Aggregation;
use crate::error::Error;

#[derive(Debug)] 
enum Upstream {
    Keybase,
    NextID,
    SybilList,
    Aggregation,
}

struct upstreamFactory;

impl upstreamFactory {
    fn new_fetcher(u: &Upstream, platform: String, identity: String) -> Box<dyn Fetcher> {
        match u {
            Upstream::Keybase => Box::new(Keybase {platform:platform.clone(), identity:identity.clone()}),
            Upstream::NextID => Box::new(ProofClient {persona:identity.clone()}),
            Upstream::SybilList => Box::new(SybilList {}),
            Upstream::Aggregation => Box::new(Aggregation {platform: platform.clone(), identity:identity.clone()}),
        }
    }
}

async fn fetcher(platform: String, identity: String) -> Result<Vec<Connection>, Error> {
    let upstreamVec: Vec<Upstream> = vec![Upstream::NextID, Upstream::Keybase, Upstream::SybilList, Upstream::Aggregation];
   
    let mut data_fetch: Box<dyn Fetcher>;
    let mut ability: Vec<(Vec<Platform>, Vec<Platform>)>;
    let mut result = Vec::new();

    for source in upstreamVec.into_iter() {
        data_fetch = upstreamFactory::new_fetcher(&source, platform.clone(), identity.clone());
        ability = data_fetch.ability();
        for (support_platforms, _) in ability.into_iter() {
            if support_platforms.iter().any(|p| p.to_string() == platform) {
                let mut res = data_fetch.fetch().await;
                if res.is_ok() {
                    result.append(& mut res.unwrap());
                } else {
                    continue;
                }             
            }
        }
    }
    return Ok(result);
}
