mod tests {
    use crate::{
        error::Error,
        graph::{new_db_connection, vertex::Identity},
        upstream::sybil_list::SybilList,
        upstream::{Fetcher, Platform},
        util::naive_now,
    };

    #[tokio::test]
    async fn test_get_sybil_result() -> Result<(), Error> {
        let sy: SybilList = SybilList {};
        sy.fetch().await?;
        let db = new_db_connection().await?;
        let addr = String::from("0x2467Ee73Bb0c5AcDeEdf4E6cC5aA685741126872");
        let found = Identity::find_by_platform_identity(&db, &Platform::Ethereum, addr.as_str())
            .await?
            .expect("Record not found");

        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());
        Ok(())
    }
}
