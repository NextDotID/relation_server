mod tests {
    use crate::{error::Error, sybil_list::query, sybil_list::SybilList};
    //const PROOF_SERVICE_URL: &str = "https://proof-service.nextnext.id"; // Staging

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let result = query(
            "0x186baD94057591918c3265C4Ddb12874324BE8Ac",
            "",
        )
        .await;
        //assert_eq!(result, 0);

        let result = match result {
            Ok(res) => {
                println!("{:?}", res);
                //println!("{:?}", res.fetch(&self, ""));

            },
            Err(error) => println!("{}", error),
        };

        Ok(())
    }
}