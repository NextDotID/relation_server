use chrono::NaiveDateTime;
use crate::error::Error;
use http::Response;
use serde::Deserialize;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client};
use hyper_tls::HttpsConnector;

/// Returns current UNIX timestamp (unit: second).
pub fn timestamp() -> i64 {
    naive_now().timestamp()
}

/// Work as `NaiveDateTime::now()`
pub fn naive_now() -> NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// Convert timestamp into NaiveDateTime struct.
pub fn timestamp_to_naive(ts: i64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(ts, 0)
}

pub fn make_client() -> Client<HttpsConnector<HttpConnector>> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    client
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

    Ok(serde_json::from_str(&body)?)
}

/// A chrono::DateTime with default
pub struct DateTimeDefault(pub chrono::DateTime<chrono::offset::Utc>);
impl Default for DateTimeDefault {
    fn default() -> DateTimeDefault {
        DateTimeDefault(chrono::Utc::now())
    }
}
