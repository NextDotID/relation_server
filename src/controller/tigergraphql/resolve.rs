use crate::{
    error::{Error, Result},
    tigergraph::{
        delete_domain_collection, delete_graph_inner_connection,
        edge::{resolve::ResolveReverse, AvailableDomain, Resolve, ResolveEdge},
        vertex::{DomainCollection, IdentityRecord},
    },
    upstream::{
        fetch_all, fetch_domains, trim_name, Chain, ContractCategory, DataFetcher, DataSource,
        DomainNameSystem, DomainStatus, Platform, Target,
    },
    util::{make_http_client, naive_now},
};
use async_graphql::{Context, Object};
use strum::IntoEnumIterator;
use tokio::time::{sleep, Duration};
use tracing::{event, Level};
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
        self.updated_at.and_utc().timestamp()
    }

    /// `reverse`: Return `True` or `False`. Show domain is primary domain or not.
    async fn reverse(&self) -> bool {
        self.reverse.clone()
    }
}

#[Object]
impl AvailableDomain {
    /// Platform.  See `avaliablePlatforms` or schema definition for a
    /// list of platforms supported by RelationService.
    async fn platform(&self) -> Platform {
        self.platform
    }

    /// Name of domain (e.g., `vitalik.eth`, `dotbit.bit`)
    async fn name(&self) -> String {
        self.name.clone()
    }

    /// `expiredAt` Expiration time of this domain name
    async fn expired_at(&self) -> Option<i64> {
        self.expired_at.map(|dt| dt.and_utc().timestamp())
    }

    /// availability is `true` means that the domain is available for registration
    /// availability is `false` means that the domain has taken by some wallet
    async fn availability(&self) -> bool {
        self.availability.clone()
    }

    /// status: taken/protected/available
    async fn status(&self) -> DomainStatus {
        self.status.clone()
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
        self.updated_at.and_utc().timestamp()
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

    /// `expiredAt` Expiration time of this domain name
    async fn expired_at(&self) -> Option<i64> {
        self.expired_at.map(|dt| dt.and_utc().timestamp())
    }
}

#[derive(Default)]
pub struct ResolveQuery {}

#[Object]
impl ResolveQuery {
    async fn available_name_system(&self) -> Result<Vec<DomainNameSystem>> {
        Ok(DomainNameSystem::iter().collect())
    }

    #[tracing::instrument(level = "trace", skip(self, _ctx))]
    async fn domain_available_search(
        &self,
        _ctx: &Context<'_>,
        #[graphql(
            desc = "name, providing name to query the registration of each domain system. See `availableNameSystem` for all domain name system supported by RelationService."
        )]
        name: String,
    ) -> Result<Option<Vec<AvailableDomain>>> {
        let process_name = trim_name(&name);
        let client = make_http_client();
        // Check name if exists in storage
        match DomainCollection::domain_available_search(&client, &process_name).await? {
            None => {
                let fetch_result = fetch_domains(&process_name).await;
                if fetch_result.is_err() {
                    event!(
                        Level::WARN,
                        process_name,
                        err = fetch_result.unwrap_err().to_string(),
                        "Failed to fetch_domains"
                    );
                }
                match DomainCollection::domain_available_search(&client, &process_name).await? {
                    None => Ok(None),
                    Some(result) => Ok(Some(result.domains)),
                }
            }
            Some(found) => {
                // filter out dataSource == "basenames" edges
                let filter_edges: Vec<AvailableDomain> = found
                    .domains
                    .clone()
                    .into_iter()
                    .filter(|e| e.platform != Platform::Basenames && e.availability == false)
                    .collect();

                if filter_edges.len() == 0 {
                    // only have basenames edges
                    let updated_at = found.collection.updated_at.clone();
                    let current_time = naive_now();
                    let duration_since_update = current_time.signed_duration_since(updated_at);
                    // Check if the difference is greater than 2 hours
                    if duration_since_update > chrono::Duration::hours(2) {
                        event!(
                            Level::DEBUG,
                            process_name,
                            "Outdated. Delete and Refetching all available domains."
                        );
                        delete_domain_collection(&client, &process_name).await?;
                        fetch_domains(&name).await?;
                        match DomainCollection::domain_available_search(&client, &process_name)
                            .await?
                        {
                            None => return Ok(None),
                            Some(result) => return Ok(Some(result.domains)),
                        }
                    }
                } else {
                    if found.collection.is_outdated() {
                        event!(
                            Level::DEBUG,
                            process_name,
                            "Outdated. Delete and Refetching all available domains."
                        );
                        let client_clone = client.clone();
                        tokio::spawn(async move {
                            // Delete and Refetch in the background
                            sleep(Duration::from_secs(10)).await;
                            delete_domain_collection(&client_clone, &process_name).await?;
                            fetch_domains(&name).await?;
                            Ok::<_, Error>(())
                        });
                    }
                }

                Ok(Some(found.domains))
            }
        }
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
                            let v_id: String = resolve
                                .clone()
                                .owner
                                .and_then(|f| Some(f.v_id.clone()))
                                .unwrap_or("".to_string());
                            tokio::spawn(async move {
                                // Delete and Refetch in the background
                                sleep(Duration::from_secs(10)).await;
                                delete_graph_inner_connection(&client, v_id).await?;
                                fetch_all(vec![target], Some(3)).await?;
                                Ok::<_, Error>(())
                            });
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
                            let v_id: String = resolve
                                .clone()
                                .owner
                                .and_then(|f| Some(f.v_id.clone()))
                                .unwrap_or("".to_string());
                            tokio::spawn(async move {
                                // Delete and Refetch in the background
                                sleep(Duration::from_secs(10)).await;
                                delete_graph_inner_connection(&client, v_id).await?;
                                fetch_all(vec![target], Some(3)).await?;
                                Ok::<_, Error>(())
                            });
                        }
                        Ok(Some(resolve))
                    }
                }
            }
            _ => Ok(None),
        }
    }
}
