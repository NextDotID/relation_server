mod tests;

use gremlin_client::{GremlinClient, Vertex};
use crate::error::Error;

// TODO: should take URL from config.
const GREMLIN_URL: &str = "localhost";

pub fn connect() -> Result<(), Error> {
    let client = GremlinClient::connect(GREMLIN_URL)?;
    // let vertex = Vertex::new("g.addV('person').property('name', 'marko').property('age', 29)");
    // let result = client.submit(vertex)?;
    let results = client
        .execute("g.V(param)", &[("param", &1)])?
        .filter_map(Result::ok)
        .map(|f| f.take::<Vertex>())
        .collect::<Result<Vec<Vertex>, _>>()?;

    println!("{:?}", results);
    Ok(())
}
