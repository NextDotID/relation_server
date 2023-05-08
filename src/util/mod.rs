#[cfg(test)]
mod tests;

use std::{collections::HashSet, hash::Hash};

use crate::error::Error;
use chrono::NaiveDateTime;
use http::Response;
use hyper::{body::HttpBody as _, client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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
    // let mut http = HttpConnector::new();
    // http.set_connect_timeout(Some(std::time::Duration::from_secs(5)));
    // let https = HttpsConnector::new_with_connector(http);

    Client::builder().build::<_, hyper::Body>(https)
}

pub fn make_http_client() -> Client<HttpConnector> {
    let mut http = HttpConnector::new();
    // tigergraphdb default idle timeout is 16 seconds
    http.set_connect_timeout(Some(std::time::Duration::from_secs(30)));
    Client::builder().build::<_, hyper::Body>(http)
}

/// If timeout is None, default timeout is 5 seconds.
pub async fn request_with_timeout(
    client: &Client<HttpsConnector<HttpConnector>>,
    req: Request<Body>,
    timeout: Option<std::time::Duration>,
) -> Result<Response<Body>, Error> {
    match tokio::time::timeout(timeout.unwrap_or(DEFAULT_TIMEOUT), client.request(req)).await {
        Ok(resp) => match resp {
            Ok(resp) => Ok(resp),
            Err(err) => Err(Error::General(
                format!("error: {:?}", err),
                lambda_http::http::StatusCode::BAD_REQUEST,
            )),
        },
        Err(_) => Err(Error::General(
            format!(
                "Timeout: no response in {:?}.",
                timeout.unwrap_or(DEFAULT_TIMEOUT)
            ),
            lambda_http::http::StatusCode::REQUEST_TIMEOUT,
        )),
    }
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

pub(crate) fn hashset_append<T>(set: &mut HashSet<T>, items: Vec<T>)
where
    T: Eq + Clone + Hash,
{
    for i in items {
        set.insert(i);
    }
}

pub fn option_naive_datetime_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_s = Option::<String>::deserialize(deserializer)?;
    match opt_s {
        Some(s) => {
            let dt = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .map_err(serde::de::Error::custom)?;
            Ok(Some(dt))
        }
        None => Ok(None),
    }
}

pub fn option_naive_datetime_to_string<S>(
    dt: &Option<NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => {
            let s = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            Serialize::serialize(&s, serializer)
        }
        None => serializer.serialize_none(),
    }
}

pub fn naive_datetime_from_string<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)
}

pub fn naive_datetime_to_string<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted = dt.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted)
}
