mod tests;

use gremlin_client::{aio::GremlinClient, Vertex};
use tokio_stream::StreamExt;
use crate::error::Error;

// TODO: should take URL from config.
const GREMLIN_URL: &str = "localhost";

pub async fn connect() -> Result<(), Error> {
    let client = GremlinClient::connect(GREMLIN_URL).await?;
    let results = client.execute("g.V(param)", &[("param", &1)]).await?
        .filter_map(Result::ok)
        .map(|f| f.take::<Vertex>())
        .collect::<Result<Vec<Vertex>, _>>().await?;
    println!("{:?}", results);
    Ok(())

}
