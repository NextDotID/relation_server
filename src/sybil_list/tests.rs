mod tests {
    use crate::{error::Error, sybil_list::SybilList, upstream::Fetcher};
    //const PROOF_SERVICE_URL: &str = "https://proof-service.nextnext.id"; // Staging

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let sy: SybilList = SybilList {};
        let result = sy.fetch(Some(" ".to_string())).await;
        let result = match result {
            Ok(res) => {
                let mut c = 1;
                for i in res.iter() {
                    c = c+1;
                    if c > 2 {
                        break;
                    }
                    println!("{:?}", i);
                }
                //println!("{:?}", res.fetch(&self, ""));
            },
            Err(error) => println!("{}", error),
        };

        Ok(())
    }
}