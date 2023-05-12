use crate::{
    error::{Error, Result},
    graph::{
        edge::{resolve::ResolveEdge, Resolve},
        vertex::IdentityRecord,
        ConnectionPool,
    },
    upstream::{
        fetch_all, Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, Target,
    },
};
use async_graphql::{Context, Object};
use strum::IntoEnumIterator;
use tracing::debug;
use uuid::Uuid;

#[Object]
impl ResolveEdge {
    /// UUID of this record.
    async fn uuid(&self) -> Uuid {
        // self.uuid
        self.record.uuid
    }

    /// Data source (upstream) which provides this info.
    async fn source(&self) -> DataSource {
        self.source
    }

    /// Domain Name system
    async fn system(&self) -> DomainNameSystem {
        self.system
    }

    /// Name of domain (e.g., `vitalik.eth`, `dotbit.bit`)
    async fn name(&self) -> String {
        self.name.clone()
    }

    /// Who collects this data.
    /// It works as a "data cleansing" or "proxy" between `source`s and us.
    async fn fetcher(&self) -> DataFetcher {
        self.fetcher
    }

    /// When this connection is fetched by us RelationService.
    async fn updated_at(&self) -> i64 {
        self.updated_at.timestamp()
    }

    /// `resolved`: Find an Ethereum wallet using ENS name or .bit alias.
    async fn resolved(&self) -> Option<IdentityRecord> {
        self.resolved.clone()
    }

    /// `owner`: Return ENS name or .bit owned by wallet address.
    async fn owner(&self) -> Result<IdentityRecord> {
        match self.owner.clone() {
            None => Err(Error::GraphQLError("owner no found.".to_string())),
            Some(owner) => Ok(owner),
        }
    }
}

#[derive(Default)]
pub struct ResolveQuery {}

#[Object]
impl ResolveQuery {
    async fn available_name_system(&self) -> Vec<String> {
        DomainNameSystem::iter()
            .map(|system| system.to_string())
            .collect()
    }

    async fn domain(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "What kind of domain name system is. See `availableNameSystem` for all domain name system supported by RelationService."
        )]
        domain_system: DomainNameSystem,
        #[graphql(
            desc = "Name of domain. For example the name is (name: \"abc.eth\") or (name: \"abc.bit\") or (name: \"abc.bnb\")"
        )]
        name: String,
    ) -> Result<Option<ResolveEdge>> {
        let pool: &ConnectionPool = ctx.data().map_err(|err| Error::PoolError(err.message))?;
        debug!("Connection pool status: {:?}", pool.status());

        match domain_system {
            DomainNameSystem::ENS => {
                let target = Target::NFT(
                    Chain::Ethereum,
                    ContractCategory::ENS,
                    ContractCategory::ENS.default_contract_address().unwrap(),
                    name.clone(),
                );
                match Resolve::find_by_ens_name(&pool, &name).await? {
                    None => {
                        let _ = fetch_all(target).await;
                        Resolve::find_by_ens_name(&pool, &name).await
                    }
                    Some(resolve) => {
                        if resolve.is_outdated() {
                            tokio::spawn(fetch_all(target));
                        }
                        Ok(Some(resolve))
                    }
                }
            }
            DomainNameSystem::DotBit
            | DomainNameSystem::Lens
            | DomainNameSystem::UnstoppableDomains
            | DomainNameSystem::SpaceId => {
                let platform = domain_system.into();
                let target = Target::Identity(platform, name.clone());
                match Resolve::find_by_domain_platform_name(&pool, &name, &domain_system, &platform)
                    .await?
                {
                    None => {
                        let _ = fetch_all(target).await;
                        Resolve::find_by_domain_platform_name(
                            &pool,
                            &name,
                            &domain_system,
                            &platform,
                        )
                        .await
                    }
                    Some(resolve) => {
                        if resolve.is_outdated() {
                            tokio::spawn(fetch_all(target));
                        }
                        Ok(Some(resolve))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}
