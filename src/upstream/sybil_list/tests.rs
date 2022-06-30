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

<<<<<<< HEAD
    #[tokio::test]
    async fn test_sybil_ability() -> Result<(), Error> {
        let result = SybilList::ability();
        println!("{:?}", result);
        assert_ne!(result.len(), 0);
=======
        assert_eq!(found.updated_at.timestamp(), naive_now().timestamp());
>>>>>>> 78e305a2b365901e86fbd0f70a72b93a470df603
        Ok(())
    }
}
