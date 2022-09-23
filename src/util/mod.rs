#[cfg(test)]
mod tests;

use std::{
    collections::HashSet,
    hash::Hash,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use crate::error::Error;
use chrono::NaiveDateTime;
use http::Response;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;
use serde::Deserialize;

/// Returns current UNIX timestamp (unit: second).
pub fn timestamp() -> i64 {
    naive_now().timestamp()
}

/// Work as `NaiveDateTime::now()`
pub fn naive_now() -> NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// Parse `String` type, second-based timestamp to NaiveDateTime
pub fn parse_timestamp(timestamp: &str) -> Result<NaiveDateTime, Error> {
    let timestamp: i64 = timestamp.parse()?;
    Ok(timestamp_to_naive(timestamp, 0))
}

/// Convert timestamp into NaiveDateTime struct.
pub fn timestamp_to_naive(ts: i64, ms: u32) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(ts, ms * 1000000)
}

pub fn make_client() -> Client<HttpsConnector<HttpConnector>> {
    let https = HttpsConnector::new();

    Client::builder().build::<_, hyper::Body>(https)
}

pub async fn parse_body<T>(resp: &mut Response<Body>) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut body_bytes: Vec<u8> = vec![];
    while let Some(chunk) = resp.body_mut().data().await {
        let mut chunk_bytes = chunk.unwrap().to_vec();
        body_bytes.append(&mut chunk_bytes);
    }
    let body = std::str::from_utf8(&body_bytes).unwrap();

    Ok(serde_json::from_str(body)?)
}

pub(crate) fn hashset_pop<T>(set: &mut HashSet<T>) -> Option<T>
where
    T: Eq + Hash + Clone,
{
    if set.is_empty() {
        None
    } else {
        let elt = set.iter().next().cloned().unwrap();
        Some(set.take(&elt).unwrap())
    }
}

pub(crate) fn hashset_append<T>(set: &mut HashSet<T>, items: Vec<T>)
where
    T: Eq + Clone + Hash,
{
    for i in items {
        set.insert(i);
    }
}
