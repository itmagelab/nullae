pub mod consul;

use crate::prelude::*;

/// Storage trait defining the interface for all storage operations.
/// Implementations can use different backends (Consul, Redis, PostgreSQL, etc.).
#[async_trait::async_trait]
pub trait Storage {
    /// Builds a URL for the given path
    fn build_url(&self, path: &str) -> String;

    /// Gets entities by URL (internal helper method)
    async fn get_by_url(&self, url: &str) -> anyhow::Result<Vec<Entity>>;

    /// Finds entities by hash pattern (internal helper method)
    async fn find_by_entity(&self, hash: &str) -> anyhow::Result<Vec<Entity>>;

    /// Gets an entity by its path
    async fn get(&self, entity: &Entity) -> anyhow::Result<Option<Entity>>;

    /// Saves or updates an entity
    async fn put(&self, entity: &Entity) -> anyhow::Result<Entity>;

    /// Creates an entity (returns existing if already exists)
    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity>;

    /// Finds entity by hash
    async fn find_by_hash(&self, hash: &str) -> anyhow::Result<Option<Entity>>;

    /// Finds entity by slug
    async fn find_by_slug(&self, slug: &str) -> anyhow::Result<Option<Entity>>;

    /// Finds entities by index
    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>>;

    /// Finds entities by pattern (hash, slug, or index)
    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>>;

    /// Lists all entities
    async fn list(&self) -> anyhow::Result<Vec<Entity>>;

    /// Gets raw HTTP client pool for low-level operations
    fn pool(&self) -> &reqwest::Client;

    /// Gets base URL for low-level operations
    fn url(&self) -> &str;
}
