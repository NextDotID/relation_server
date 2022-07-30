#[cfg(test)]
mod tests {
    use crate::graph::new_db_connection;

    #[tokio::test]
    async fn test_new_db_connection() {
        assert!(!new_db_connection()
            .await
            .unwrap()
            .collections_names()
            .is_empty())
    }
}
