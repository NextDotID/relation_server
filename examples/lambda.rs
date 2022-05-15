use lambda_http::{service_fn, Error as LambdaError};


#[tokio::main]
async fn main() -> Result<(), LambdaError> {
//     let _ = env_logger::try_init();

//     lambda_http::run(service_fn(entrypoint)).await?;
    Ok(())
}

// fn entrypoint (req: LambdaRequest) -> Result<impl IntoResponse, LambdaError> {
//     todo!()
// }

// /// Translate between `lambda_http` `Body` and our `Body`.
// async fn parse<F>(req: LambdaRequest, controller: fn(OurRequest) -> F) -> LambdaResponse<LambdaBody>
// where
//     F: Future<Output = Result<OurResponse, Error>>,
// {
//     let (parts, old_body) = req.into_parts();
//     let body: OurBody = crate::controller::LambdaBody(old_body).into();
//     let new_req: OurRequest = http::Request::from_parts(parts, body);

//     match controller(new_req).await {
//         Ok(resp) => {
//             let (parts, our_resp) = resp.into_parts();
//             let resp = lambda_http::Body::Text(our_resp);
//             LambdaResponse::from_parts(parts, resp)
//         }
//         Err(err) => {
//             let (parts, our_resp) = error_response(err).into_parts();
//             let resp = lambda_http::Body::Text(our_resp);
//             LambdaResponse::from_parts(parts, resp)
//         }
//     }
// }
