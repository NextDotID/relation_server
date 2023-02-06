use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use dataloader::non_cached::Loader;
use http::StatusCode;
use relation_server::{
    config::{self, C},
    controller::graphql::Query,
    error::Result,
    graph::arangopool::new_connection_pool,
    graph::vertex::contract::ContractLoadFn,
    graph::vertex::FromToLoadFn,
    graph::vertex::IdentityLoadFn,
};
// use aragog::{AuthMode, DatabaseConnection, OperationOptions};
use std::{convert::Infallible, net::SocketAddr};
use tracing::{info, warn};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use warp::{http::Response as HttpResponse, Filter, Rejection};

#[tokio::main]
async fn main() -> Result<()> {
    let log_subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive("hyper=info".parse().unwrap()),
        )
        .finish();

    tracing::subscriber::set_global_default(log_subscriber)
        .expect("Setting default subscriber failed");

    let middleware_cors = warp::cors()
        .allow_any_origin() // : maybe more strict CORS in production?
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["Accept", "Content-Type", "Length"]);

    // Performing DB migration
    let _db = aragog::DatabaseConnection::builder()
        .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
        .with_auth_mode(aragog::AuthMode::Basic)
        .with_operation_options(aragog::OperationOptions::default())
        .with_schema_path(&C.db.schema_path)
        .apply_schema() // Only apply database migration here.
        .build()
        .await?;

    // Runtime::Tokio1
    let pool = new_connection_pool().await?;
    let contract_loader_fn = ContractLoadFn {
        pool: pool.to_owned(),
    };
    let identity_loader_fn = IdentityLoadFn {
        pool: pool.to_owned(),
    };
    let from_to_loader_fn = FromToLoadFn {
        pool: pool.to_owned(),
    };
    // HOLD ON: Specify the batch size number
    let contract_loader = Loader::new(contract_loader_fn)
        .with_max_batch_size(100)
        .with_yield_count(10);
    let identity_loader = Loader::new(identity_loader_fn)
        .with_max_batch_size(100)
        .with_yield_count(10);
    let from_to_loader = Loader::new(from_to_loader_fn)
        .with_max_batch_size(100)
        .with_yield_count(10);

    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(pool)
        .data(contract_loader)
        .data(identity_loader)
        .data(from_to_loader)
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
    info!("Playground: http://{}", address);

    warp::serve(routes).run(address).await;

    println!("Shutting down...");
    Ok(())
}
