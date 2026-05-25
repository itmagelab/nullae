pub mod consul;
pub mod memory;
pub mod etcd;

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

    fn purge_index(
        &self,
        entity: &Entity,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

#[derive(Debug)]
pub enum StorageBackend {
    Consul(consul::Consul),
    Etcd(etcd::Etcd),
}

impl StorageBackend {
    pub fn new() -> anyhow::Result<Self> {
        if let Ok(etcd_url) = std::env::var("NULLAE_ETCD_URL") {
            if !etcd_url.is_empty() {
                let storage = etcd::Etcd::new()?;
                return Ok(Self::Etcd(storage));
            }
        }

        if let Ok(backend) = std::env::var("NULLAE_STORAGE_BACKEND") {
            match backend.as_str() {
                "etcd" => {
                    let storage = etcd::Etcd::new()?;
                    return Ok(Self::Etcd(storage));
                }
                "consul" => {
                    let storage = consul::Consul::new()?;
                    return Ok(Self::Consul(storage));
                }
                _ => {}
            }
        }

        // Default fallback to Consul
        let storage = consul::Consul::new()?;
        Ok(Self::Consul(storage))
    }
}

impl Storage for StorageBackend {
    async fn get_index(&self, path: &str) -> anyhow::Result<Option<Item>> {
        match self {
            Self::Consul(c) => c.get_index(path).await,
            Self::Etcd(e) => e.get_index(path).await,
        }
    }

    async fn save(&self, entity: &Entity) -> anyhow::Result<Entity> {
        match self {
            Self::Consul(c) => c.save(entity).await,
            Self::Etcd(e) => e.save(entity).await,
        }
    }

    async fn save_index(&self, path: &str, item: &Item) -> anyhow::Result<()> {
        match self {
            Self::Consul(c) => c.save_index(path, item).await,
            Self::Etcd(e) => e.save_index(path, item).await,
        }
    }

    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity> {
        match self {
            Self::Consul(c) => c.create(entity).await,
            Self::Etcd(e) => e.create(entity).await,
        }
    }

    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        match self {
            Self::Consul(c) => c.find(pattern).await,
            Self::Etcd(e) => e.find(pattern).await,
        }
    }

    async fn get(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        match self {
            Self::Consul(c) => c.get(hash).await,
            Self::Etcd(e) => e.get(hash).await,
        }
    }

    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        match self {
            Self::Consul(c) => c.find_by_index(index, pattern).await,
            Self::Etcd(e) => e.find_by_index(index, pattern).await,
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<Entity>> {
        match self {
            Self::Consul(c) => c.list().await,
            Self::Etcd(e) => e.list().await,
        }
    }

    async fn delete_index(&self, path: &str) -> anyhow::Result<()> {
        match self {
            Self::Consul(c) => c.delete_index(path).await,
            Self::Etcd(e) => e.delete_index(path).await,
        }
    }

    async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        match self {
            Self::Consul(c) => c.delete(entity).await,
            Self::Etcd(e) => e.delete(entity).await,
        }
    }

    async fn purge_index(&self, entity: &Entity) -> anyhow::Result<()> {
        match self {
            Self::Consul(c) => c.purge_index(entity).await,
            Self::Etcd(e) => e.purge_index(entity).await,
        }
    }
}

