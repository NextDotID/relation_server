pub mod graphql;
pub mod healthz;

use crate::upstream::Platform;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Deref};

use crate::error::Error;

pub mod lambda;

pub type Body = String;
pub struct LambdaBody(lambda_http::Body);
impl Deref for LambdaBody {
    type Target = lambda_http::Body;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Request = http::Request<Body>;
pub type Response = http::Response<Body>;

impl From<LambdaBody> for Body {
    fn from(body: LambdaBody) -> Self {
        match body.0 {
            lambda_http::Body::Empty => "".into(),
            lambda_http::Body::Text(text) => text,
            lambda_http::Body::Binary(bitstring) => String::from_utf8(bitstring).unwrap(),
        }
    }
}

impl From<Body> for LambdaBody {
    fn from(body: Body) -> Self {
        LambdaBody(lambda_http::Body::Text(body))
    }
}

// MARK: Helper fn

pub fn json_parse_body<T>(req: &Request) -> Result<T, Error>
where
    for<'de> T: Deserialize<'de>,
{
    serde_json::from_str(req.body()).map_err(|e| e.into())
}

pub fn json_response<T>(status: StatusCode, resp: &T) -> Result<Response, Error>
where
    T: Serialize,
{
    let body = serde_json::to_string(resp).unwrap();

    http::Response::builder()
        .status(status)
        // CORS
        // TODO: impl this with tower middleware
        .header("Access-Control-Allow-Origin", "*")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type,Authorization,X-Amz-Date,X-Api-Key,X-Amz-Security-Token",
        )
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .body(body)
        .map_err(|e| e.into())
}

pub fn query_parse(req: Request) -> HashMap<String, String> {
    req.uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new)
}

pub fn vec_string_to_vec_platform(vec_string: Vec<String>) -> Result<Vec<Platform>, Error> {
    let platforms_result: Result<Vec<Platform>, _> = vec_string
        .into_iter()
        .map(|p_string| p_string.parse())
        .collect();
    Ok(platforms_result?)
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    pub message: String,
}

pub fn error_response(err: Error) -> Response {
    let resp = ErrorResponse {
        message: err.to_string(),
    };
    let body: String = serde_json::to_string(&resp).unwrap();

    http::Response::builder()
        .status(err.http_status())
        .header("Access-Control-Allow-Origin", "*")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type,Authorization,X-Amz-Date,X-Api-Key,X-Amz-Security-Token",
        )
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .body(body)
        .expect("failed to render response")
}
