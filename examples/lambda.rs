use relation_server::controller::lambda::entrypoint;
use lambda_http::{service_fn, Error as LambdaError};

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    let _ = env_logger::try_init();

    lambda_http::run(service_fn(entrypoint)).await?;
    Ok(())
}
