use crate::graph::Vertex;
use async_trait::async_trait;
use gremlin_client::{
    aio::AsyncTerminator, process::traversal::GraphTraversalSource, GremlinError, GID,
    derive::{FromGMap, FromGValue},
};

const LABEL: &str = "identity";

#[derive(Debug, Clone, FromGMap, FromGValue)]
pub struct Identity {
    pub uuid: uuid::Uuid,
    pub platform: String,
    pub identity: String,
    pub display_name: String,
    pub created_at: chrono::DateTime<chrono::offset::Utc>,
}

#[async_trait]
impl Vertex for Identity {
    fn label(&self) -> &'static str {
        LABEL
    }

    async fn save(&self, g: &GraphTraversalSource<AsyncTerminator>) -> Result<GID, GremlinError> {
        let created = g.add_v(LABEL)
            .property("uuid", self.uuid.clone())
            .property("platform", &self.platform)
            .property("identity", &self.identity)
            .property("display_name", &self.display_name)
            .property("created_at", self.created_at.clone())
            .to_list().await?;

        Ok(created.first().expect("Should have at least 1").id().clone())
    }

    async fn find(
        g: &GraphTraversalSource<AsyncTerminator>,
        platform: &str,
        identity: &str,
    ) -> Result<Vec<Identity>, GremlinError> {
        let result: Vec<Identity> = g
            .v(())
            .has_label(LABEL)
            .has(("platform", platform))
            .has(("identity", identity))
            .value_map(())
            .to_list().await?
            .into_iter()
            .map(Identity::try_from)
            .filter_map(Result::ok)
            .collect();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{error::Error, graph::create_traversal};
    use fake::{Fake, Faker};
    use gremlin_client::GValue;

    #[tokio::test]
    async fn test_save_find() -> Result<(), Error> {
        let identity = Identity {
            uuid: uuid::Uuid::new_v4(),
            platform: "twitter".to_string(),
            identity: Faker.fake(),
            display_name: Faker.fake(),
            created_at: chrono::Utc::now(),
        };
        let g = create_traversal().await?;
        let created_id = identity.save(&g).await?;
        println!("{:?}", created_id);

        let uuids = g.v(()).values("uuid").to_list().await?;
        println!("uuid: {:?}", uuids);
        assert!(uuids.len() > 0);
        assert!(uuids.contains(&GValue::Uuid(identity.uuid)));
        Ok(())
    }

    #[tokio::test]
    async fn test_find() -> Result<(), Error> {
        let identity = Identity {
            uuid: uuid::Uuid::new_v4(),
            platform: "twitter".to_string(),
            identity: Faker.fake(),
            display_name: Faker.fake(),
            created_at: chrono::Utc::now(),
        };
        let g = create_traversal().await?;
        identity.save(&g).await?;

        let found = Identity::find(&g, &identity.platform, &identity.identity).await?;
        assert_eq!(found.len(), 1);
        let found_identity = found.first().expect("Should have at least 1").clone();
        assert_eq!(found_identity.uuid, identity.uuid);

        Ok(())
    }
}
