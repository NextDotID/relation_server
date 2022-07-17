use aragog::{AuthMode, DatabaseConnection, OperationOptions};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use env_logger::Env;
use http::StatusCode;
use log::warn;
use relation_server::{
    config::{self, C},
    controller::graphql::Query,
    error::Result,
};
use std::{convert::Infallible, net::SocketAddr};
use warp::{http::Response as HttpResponse, Filter, Rejection};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .try_init()
        .expect("Failed to initialize logger");

    let db = DatabaseConnection::builder()
        .with_credentials(&C.db.host, &C.db.db, &C.db.username, &C.db.password)
        .with_auth_mode(AuthMode::Basic)
        .with_operation_options(OperationOptions::default())
        .with_schema_path(&C.db.schema_path)
        .apply_schema() // Only apply database migration here.
        .build()
        .await?;
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(db)
        .finish();

    let graphql_post = async_graphql_warp::graphql(schema).and_then(
        |(schema, request): (
            Schema<Query, EmptyMutation, EmptySubscription>,
            async_graphql::Request,
        )| async move {
            Ok::<_, Infallible>(GraphQLResponse::from(schema.execute(request).await))
        },
    );

    let graphql_playground = warp::path::end().and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/")))
    });

    let routes = graphql_playground
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
    println!("Playground: http://{}", address.to_string());
    warp::serve(routes).run(address).await;

    println!("Shutting down...");
    Ok(())
}
