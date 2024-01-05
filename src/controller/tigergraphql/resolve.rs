use crate::{
    error::{Error, Result},
    tigergraph::{
        edge::{resolve::ResolveReverse, Resolve, ResolveEdge},
        vertex::IdentityRecord,
    },
    upstream::{
        fetch_all, Chain, ContractCategory, DataFetcher, DataSource, DomainNameSystem, Target,
    },
    util::make_http_client,
};
use async_graphql::{Context, Object};
use strum::IntoEnumIterator;
use uuid::Uuid;

#[Object]
impl ResolveReverse {
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

    /// `reverse`: Return `True` or `False`. Show domain is primary domain or not.
    async fn reverse(&self) -> bool {
        self.reverse.clone()
    }
}

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

    /// `reverse`: Return `True` or `False`. Show domain is primary domain or not.
    async fn reverse(&self) -> bool {
        self.reverse.clone()
    }

    /// `reverseRecord`: Only have one primary domain linked to an address.
    async fn reverse_record(&self) -> Option<IdentityRecord> {
        self.reverse_record.clone()
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

    #[tracing::instrument(level = "trace", skip(self, _ctx))]
    async fn domain(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "What kind of domain name system is. See `availableNameSystem` for all domain name system supported by RelationService."
        )]
        domain_system: DomainNameSystem,
        #[graphql(
            desc = "Name of domain. For example the name is (name: \"abc.eth\") or (name: \"abc.bit\") or (name: \"abc.bnb\")"
        )]
        name: String,
    ) -> Result<Option<ResolveEdge>> {
        let client = make_http_client();
        match domain_system {
            DomainNameSystem::ENS => {
                let target = Target::NFT(
                    Chain::Ethereum,
                    ContractCategory::ENS,
                    ContractCategory::ENS.default_contract_address().unwrap(),
                    name.clone(),
                );
                match Resolve::find_by_name_system(&client, &name, &domain_system).await? {
                    None => {
                        let _ = fetch_all(vec![target], Some(3)).await;
                        Resolve::find_by_name_system(&client, &name, &domain_system).await
                    }
                    Some(resolve) => {
                        if resolve.is_outdated() {
                            let _vid: String = resolve
                                .clone()
                                .owner
                                .and_then(|f| Some(f.v_id.clone()))
                                .unwrap_or("".to_string());
                            // Refetch in the background
                            tokio::spawn(fetch_all(vec![target], Some(3)));
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
                match Resolve::find_by_name_system(&client, &name, &domain_system).await? {
                    None => {
                        let _ = fetch_all(vec![target], Some(3)).await;
                        Resolve::find_by_name_system(&client, &name, &domain_system).await
                    }
                    Some(resolve) => {
                        if resolve.is_outdated() {
                            let _vid: String = resolve
                                .clone()
                                .owner
                                .and_then(|f| Some(f.v_id.clone()))
                                .unwrap_or("".to_string());
                            // Refetch in the background
                            tokio::spawn(fetch_all(vec![target], Some(3)));
                        }
                        Ok(Some(resolve))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}
