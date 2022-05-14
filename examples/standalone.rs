use hyper::service::{make_service_fn, service_fn};
use hyper::{
    Body as HyperBody,
    Method,
    Response as HyperResponse,
    Server,
    StatusCode,
};
use juniper::{RootNode, EmptyMutation, EmptySubscription};
use relation_server::controller::graphql::Context;
use relation_server::config::C;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();
    let config = C.clone(); // TODO

    let addr: SocketAddr = format!("{}:{}", config.web.listen, config.web.port)
        .parse()
        .expect("Unable to parse web listen address");
    let graphql_root_node = Arc::new(RootNode::new(
        relation_server::controller::graphql::Query,
        EmptyMutation::<relation_server::controller::graphql::Context>::new(),
        EmptySubscription::<relation_server::controller::graphql::Context>::new(),
    ));
    let db = Arc::new(Context { pool: "TODO".into() });

    let make_svc = make_service_fn(move |_| {
        let root_node = graphql_root_node.clone();
        let ctx = db.clone();

        async {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let root_node = root_node.clone();
                let ctx = ctx.clone();
                async {
                    Ok::<_, Infallible>(match (req.method(), req.uri().path()) {
                        (&Method::OPTIONS, _) => {
                            let mut res = HyperResponse::new(HyperBody::empty());
                            *res.status_mut() = StatusCode::NO_CONTENT;
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
                                hyper::header::HeaderValue::from_static("GET, POST, OPTIONS"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                hyper::header::HeaderValue::from_static("*"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
                                hyper::header::HeaderValue::from_static("Content-Type"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                                hyper::header::HeaderValue::from_static("true"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_MAX_AGE,
                                hyper::header::HeaderValue::from_static("3600"),
                            );
                            res
                        },
                        (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
                        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                            let mut res = juniper_hyper::graphql(root_node, ctx, req).await;
                            res.headers_mut().insert(
                                    hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
                                    hyper::header::HeaderValue::from_static("GET, POST, OPTIONS"),
                                );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                hyper::header::HeaderValue::from_static("*"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
                                hyper::header::HeaderValue::from_static("Content-Type"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                                hyper::header::HeaderValue::from_static("true"),
                            );
                            res.headers_mut().insert(
                                hyper::header::ACCESS_CONTROL_MAX_AGE,
                                hyper::header::HeaderValue::from_static("3600"),
                            );
                            res
                        },
                        _ => {
                            let mut response = HyperResponse::new("Not Found".into());
                            *response.status_mut() = StatusCode::NOT_FOUND;
                            response
                        }
                    })
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e)
    }
}
