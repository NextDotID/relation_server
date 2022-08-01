#[cfg(test)]
mod test {
    use crate::newgraph::{new_raw_db_connection};
    use crate::error::Error;
    use arangors_lite::{AqlQuery, Connection, Cursor, Database, ClientError};
    use serde_json::value::Value;
    use crate::newgraph::vertex::identity::{Identity, Path, Neighbor, Source};
    use crate::upstream::{DataSource, Platform};
    use serde_json::from_value;

    #[tokio::test]
    async fn test_arango_version() {
        println!("{:?}", new_raw_db_connection().await.unwrap().arango_version().await.unwrap().version)
        // "3.9.2"
    }

    #[tokio::test]
    async fn test_find_by_platform_identity() -> Result<(), ClientError>{
        let db = new_raw_db_connection().await?;
        let identity = "NQ5IW6W7";
        let platform= [Platform::Twitter.to_string(), Platform::Keybase.to_string()];

        let platforms =platform
            .map(|field| String::from("'") + &field + &String::from("'"))
            .join(",");
        let aql = format!(r"FOR d IN relation
        FILTER d.platform IN [{}]
        FILTER d.identity == '{}'
        RETURN d", platforms, identity);
        println!("aql = {}", aql);

        let aql = AqlQuery::new(aql.as_str())
        .batch_size(1)
        .count(true);

        let resp: Vec<Value> = db.aql_query(aql).await.unwrap();
        let mut records: Vec<Identity> = Vec::new();
        for i in resp {
            let v: Identity = from_value(i).unwrap();
            records.push(v)
        }
        println!("{:?}", records);
        Ok(())
    }

    #[tokio::test]
    async fn test_neighbors() -> Result<(), ClientError>{
        // jxocxTygjnD3s3BnawX
        let db = new_raw_db_connection().await?;
        let display_name = "or0TeHvTEMwdCQm";
        let aql = format!(r"FOR d IN relation
        SEARCH ANALYZER(d.display_name IN TOKENS('{}', 'text_en'), 'text_en')
        FOR vertex, edge, path IN 1..3 OUTBOUND d Proofs
        RETURN path", display_name);
        println!("aql={}", aql);
        let aql = AqlQuery::new(aql.as_str())
        .batch_size(1)
        .count(true);

        let resp: Vec<Value> = db.aql_query(aql).await.unwrap();
        // let mut paths: Vec<Path> = Vec::new();
        // for p in resp {
        //     let path: Path = from_value(p).unwrap();
        //     paths.push(path)
        // }

        let mut neighbors: Vec<Neighbor> = Vec::new();
        for p in resp {
            let path: Path = from_value(p).unwrap();
            let len = path.vertices.len();
            let mut sources: Vec<Source> = Vec::new();
            for proof in path.edges {
                let s = Source {
                    source: proof.source,
                    relevance: 3,
                };
                sources.push(s);
            }
            let tmp = Neighbor {
                vertex: path.vertices[len-1].to_owned(),
                sources: sources,
            };
            neighbors.push(tmp);
        }
        println!("{:?}", neighbors);
        Ok(())
    }
}