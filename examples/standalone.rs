use hyper::service::{make_service_fn, service_fn};
use hyper::{
    Body as HyperBody,
    Method,
    Request as HyperRequest,
    Response as HyperResponse,
    //  Error as HyperError,
    Server,
    StatusCode,
};
use relation_server::controller::{
    error_response, healthz, Body, Request, Response,
};
use relation_server::{config::C, error::Error};
use log::info;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();
    let config = C.clone(); // TODO

    let addr: SocketAddr = format!("{}:{}", config.web.listen, config.web.port)
        .parse()
        .expect("Unable to parse web listen address");

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(entrypoint)) });

    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e)
    }
}

async fn entrypoint(req: HyperRequest<HyperBody>) -> Result<HyperResponse<HyperBody>, Infallible> {
    info!(
        "{} {}",
        req.method().to_string(),
        req.uri().path().to_string()
    );

    Ok(match (req.method(), req.uri().path()) {
        (&Method::GET, "/healthz") => parse(req, healthz::controller).await,
        _ => HyperResponse::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not Found".into())
            .expect("Failed to render response"),
    })
}

async fn parse<F>(
    req: HyperRequest<HyperBody>,
    controller: fn(Request) -> F,
) -> HyperResponse<HyperBody>
where
    F: Future<Output = Result<Response, Error>>,
{
    let (parts, hyper_body) = req.into_parts();
    let full_body = hyper::body::to_bytes(hyper_body).await.unwrap();
    let body_string: Body = String::from_utf8(full_body.to_vec()).unwrap();

    let our_req = Request::from_parts(parts, body_string);
    match controller(our_req).await {
        Ok(resp) => {
            let (parts, our_resp) = resp.into_parts();
            let hyper_body = HyperBody::from(our_resp);
            HyperResponse::from_parts(parts, hyper_body)
        }
        Err(err) => {
            let (parts, our_resp) = error_response(err).into_parts();
            let hyper_body = HyperBody::from(our_resp);
            HyperResponse::from_parts(parts, hyper_body)
        }
    }
}
