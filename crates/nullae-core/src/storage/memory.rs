use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::prelude::*;
use crate::storage::Storage;

#[derive(Default, Debug)]
struct InMemoryStorageInner {
    entities: HashMap<String, serde_json::Value>,
    indices: HashMap<String, serde_json::Value>,
}

/// A thread-safe, in-memory storage engine that implements the `Storage` trait.
/// This acts as a complete local KV mirror of Consul's index and entity paths.
#[derive(Clone, Default, Debug)]
pub struct InMemoryStorage {
    inner: Arc<RwLock<InMemoryStorageInner>>,
}

impl InMemoryStorage {
    /// Creates a new, empty `InMemoryStorage`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InMemoryStorageInner::default())),
        }
    }
}

impl Storage for InMemoryStorage {
    async fn get_index(&self, path: &str) -> anyhow::Result<Option<Item>> {
        let inner = self.inner.read().await;
        if let Some(val) = inner.indices.get(path) {
            let item: Item = serde_json::from_value(val.clone())?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let mut inner = self.inner.write().await;
        let hash_hex = entity.hash().as_hex();
        let val = serde_json::to_value(entity)?;
        inner.entities.insert(hash_hex, val.clone());
        let saved: Entity = serde_json::from_value(val)?;
        Ok(saved)
    }

    async fn save_index(&self, path: &str, item: &Item) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let val = serde_json::to_value(item)?;
        inner.indices.insert(path.to_string(), val);
        Ok(())
    }

    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let mut inner = self.inner.write().await;
        let hash_hex = entity.hash().as_hex();
        if let Some(existing) = inner.entities.get(&hash_hex) {
            let existing_entity: Entity = serde_json::from_value(existing.clone())?;
            Ok(existing_entity)
        } else {
            let val = serde_json::to_value(entity)?;
            inner.entities.insert(hash_hex, val.clone());
            let created: Entity = serde_json::from_value(val)?;
            Ok(created)
        }
    }

    async fn get(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        let inner = self.inner.read().await;
        if let Some(val) = inner.entities.get(hash) {
            let entity: Entity = serde_json::from_value(val.clone())?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    async fn delete_index(&self, path: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        inner.indices.remove(path);
        Ok(())
    }

    async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        {
            let mut inner = self.inner.write().await;
            inner.entities.remove(&entity.hash().as_hex());
        }
        self.purge_index(entity).await?;
        Ok(())
    }

    async fn purge_index(&self, entity: &Entity) -> anyhow::Result<()> {
        let index = Index::from_entity(entity)?;
        for new_item in index.value() {
            let Some(mut saved) = self.get_index(&new_item.path()).await? else {
                continue;
            };

            saved.subtract(new_item);

            if saved.is_empty() {
                self.delete_index(&saved.path()).await?;
            } else {
                self.save_index(&saved.path(), &saved).await?;
            }
        }
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<Entity>> {
        let inner = self.inner.read().await;
        let mut list = Vec::new();
        for val in inner.entities.values() {
            let entity: Entity = serde_json::from_value(val.clone())?;
            list.push(entity);
        }
        Ok(list)
    }

    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let path = format!("{}/index/{}/{}", crate::BASE_PATH, index, pattern);
        let Some(item) = self.get_index(&path).await? else {
            return Ok(vec![]);
        };
        let uuids: HashSet<HashID> = item.value().into_iter().collect();

        let inner = self.inner.read().await;
        let mut entities = Vec::new();
        for val in inner.entities.values() {
            let entity: Entity = serde_json::from_value(val.clone())?;
            if uuids.contains(entity.hash()) {
                entities.push(entity);
            }
        }
        Ok(entities)
    }

    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let mut entities = Vec::new();
        if let Some(e) = self.get(pattern).await? {
            entities.push(e);
        }

        // Search in slugs: index is "slug/{pattern}"
        if let Some(e) = self.find_by_index("slug", pattern).await?.pop() {
            entities.push(e);
        }

        entities.extend(self.find_by_index("hostname", pattern).await?);
        entities.extend(self.find_by_index("name", pattern).await?);

        // find_by_partial_hash:
        let inner = self.inner.read().await;
        let mut partial_matches = Vec::new();
        for val in inner.entities.values() {
            let entity: Entity = serde_json::from_value(val.clone())?;
            if entity.hash().as_hex().contains(pattern) {
                partial_matches.push(entity);
            }
        }
        if partial_matches.len() > 1 {
            anyhow::bail!("duplicate entity found for hash {}", pattern);
        }
        entities.append(&mut partial_matches);

        // Deduplicate entities by hash
        let mut seen = HashSet::new();
        entities.retain(|e| seen.insert(e.hash().clone()));

        Ok(entities)
    }
}
