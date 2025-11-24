pub mod consul;

use crate::prelude::*;
use std::future::Future;

/// Storage trait defining the interface for all storage operations.
/// Implementations can use different backends (Consul, Redis, PostgreSQL, etc.).
pub trait Storage {
    /// Gets an index item by its path
    fn get_index(&self, path: &str) -> impl Future<Output = anyhow::Result<Option<Item>>> + Send;

    /// Saves or updates an entity
    fn save(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<Entity>> + Send;

    /// Puts an index item
    fn save_index(
        &self,
        path: &str,
        item: &Item,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Creates an entity (returns existing if already exists)
    fn create(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<Entity>> + Send;

    /// Finds entities by pattern (hash, slug, or index)
    fn find(&self, pattern: &str) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;

    /// Finds entity by hash
    fn get(&self, hash: &str) -> impl Future<Output = anyhow::Result<Option<Entity>>> + Send;

    /// Finds entities by index
    fn find_by_index(
        &self,
        index: &str,
        pattern: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;

    /// Lists all entities
    fn list(&self) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;

    /// Deletes an index item
    fn delete_index(&self, path: &str) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Deletes an entity
    fn delete(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<()>> + Send;
}
