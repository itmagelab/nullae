pub mod consul;

use crate::prelude::*;

/// Storage trait defining the interface for all storage operations.
/// Implementations can use different backends (Consul, Redis, PostgreSQL, etc.).
#[async_trait::async_trait]
#[async_trait::async_trait]
pub trait Storage {
    /// Gets an index item by its path
    async fn get_index(&self, path: &str) -> anyhow::Result<Option<Item>>;

    /// Saves or updates an entity
    async fn save(&self, entity: &Entity) -> anyhow::Result<Entity>;

    /// Puts an index item
    async fn save_index(&self, path: &str, item: &Item) -> anyhow::Result<()>;

    /// Creates an entity (returns existing if already exists)
    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity>;

    /// Finds entities by pattern (hash, slug, or index)
    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>>;

    /// Finds entity by hash
    async fn get(&self, hash: &str) -> anyhow::Result<Option<Entity>>;

    /// Finds entities by index
    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>>;

    /// Lists all entities
    async fn list(&self) -> anyhow::Result<Vec<Entity>>;

    /// Deletes an index item
    async fn delete_index(&self, path: &str) -> anyhow::Result<()>;

    /// Deletes an entity
    async fn delete(&self, entity: &Entity) -> anyhow::Result<()>;
}
