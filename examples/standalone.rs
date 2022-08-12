use aragog::{AuthMode, DatabaseConnection, OperationOptions};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use dataloader::non_cached::Loader;
use env_logger::Env;
use http::StatusCode;
use log::warn;
use relation_server::{
    config::{self, C},
    controller::graphql::Query,
    error::Result,
    graph::new_connection_pool,
    graph::new_raw_db_connection,
    graph::vertex::contract::ContractLoadFn,
    graph::vertex::IdentifyLoadFn,
};
use std::{convert::Infallible, net::SocketAddr};
use warp::{http::Response as HttpResponse, Filter, Rejection};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .try_init()
        .expect("Failed to initialize logger");
    let middleware_cors = warp::cors()
        .allow_any_origin() // : maybe more strict CORS in production?
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["Accept", "Content-Type", "Length"]);

    // TODO: Not sure reuse of this connection instance will cause data race.
    let db = DatabaseConnection::builder()
        .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
        .with_auth_mode(AuthMode::Basic)
        .with_operation_options(OperationOptions::default())
        .with_schema_path(&C.db.schema_path)
        .apply_schema() // Only apply database migration here.
        .build()
        .await?;
    let raw_db = new_raw_db_connection().await?;
    let pool = new_connection_pool().await;
    let contract_loader_fn = ContractLoadFn {
        pool: pool.to_owned(),
    };
    let identity_loader_fn = IdentifyLoadFn {
        pool: pool.to_owned(),
    };
    // HOLD ON: Specify the batch size number
    let contract_loader = Loader::new(contract_loader_fn)
        .with_max_batch_size(100)
        .with_yield_count(10);

    let identity_loader = Loader::new(identity_loader_fn)
        .with_max_batch_size(100)
        .with_yield_count(10);

    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(db)
        .data(raw_db)
        .data(contract_loader)
        .data(identity_loader)
        .finish();

    let graphql_post = async_graphql_warp::graphql(schema)
        .and_then(
            |(schema, request): (
                Schema<Query, EmptyMutation, EmptySubscription>,
                async_graphql::Request,
            )| async move {
                Ok::<_, Infallible>(GraphQLResponse::from(schema.execute(request).await))
            },
        )
        .with(middleware_cors);

    let playground = warp::path::end().and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/")))
    });

    let routes = playground
        .or(graphql_post)
        .recover(|err: Rejection| async move {
            if let Some(GraphQLBadRequest(err)) = err.find() {
                warn!("GraphQL error: {}", err);
                return Ok::<_, Infallible>(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }
            if let Some(myerr) = err.find::<relation_server::error::Error>() {
                warn!("General Error: {}", myerr.to_string());
                return Ok(warp::reply::with_status(
                    myerr.to_string(),
                    myerr.http_status(),
                ));
            }

            Ok(warp::reply::with_status(
                "INTERNAL_SERVER_ERROR".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        });

    let address = SocketAddr::new(config::C.web.listen.parse().unwrap(), config::C.web.port);
    println!("Playground: http://{}", address);
    warp::serve(routes).run(address).await;

    println!("Shutting down...");
    Ok(())
}
