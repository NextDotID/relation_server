mod tests {
    use crate::{error::Error, sybil_list::query, sybil_list::MatchItem};
    //const PROOF_SERVICE_URL: &str = "https://proof-service.nextnext.id"; // Staging

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let result = query(
            "0x186baD94057591918c3265C4Ddb12874324BE8AcA",
            "",
        )
        .await;
        //assert_eq!(result, 0);

        let result = match result {
            Ok(res) => println!("{:?}", res),
            Err(error) => println!("{}", error),
        };
    
        
        
        //println!("{:?}", Err(e));

        Ok(())
    }
}