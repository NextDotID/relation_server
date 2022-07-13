use crate::{
    controller::{json_response, Request, Response},
    error::Error,
};
use http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
struct HealthzResponse {
    pub hello: String,
    pub built_at: String,
    pub revision: String,
}

pub async fn controller(_req: Request) -> Result<Response, Error> {
    json_response(
        StatusCode::OK,
        &HealthzResponse {
            hello: "kv server".to_string(),
            built_at: option_env!("RELATION_SERVER_BUILT_AT")
                .unwrap_or("UNKNOWN")
                .to_string(),
            revision: option_env!("RELATION_SERVER_REVISION")
                .unwrap_or("UNKNOWN")
                .to_string(),
        },
    )
}
