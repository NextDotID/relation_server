use std::{convert::Infallible, net::SocketAddr};

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use http::StatusCode;
use relation_server::{config, controller::graphql::Query, error::Result, graph};
use warp::{http::Response as HttpResponse, Filter, Rejection};

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: not sure if sharing one DB connection instance
    let db = graph::new_db_connection().await?;

    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(db)
        .finish();

    println!("Playground: http://localhost:8000");

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
                return Ok::<_, Infallible>(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }

            Ok(warp::reply::with_status(
                "INTERNAL_SERVER_ERROR".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        });

    let address = SocketAddr::new(config::C.web.listen.parse().unwrap(), config::C.web.port);
    warp::serve(routes).run(address).await;

    println!("Shutting down...");
    Ok(())
}
