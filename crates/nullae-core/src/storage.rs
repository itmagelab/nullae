pub mod consul;

use crate::BASE_PATH;
use crate::prelude::*;
use std::future::Future;

pub trait Storage {
    fn create(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<Entity>> + Send
    where
        Self: Sync,
    {
        async move {
            if let Some(existing) = self.get(entity).await? {
                return Ok(existing);
            }

            self.put(entity).await
        }
    }

    fn get(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<Option<Entity>>> + Send
    where
        Self: Sync,
    {
        async move {
            let hash = &entity.hashid().as_hex();
            self.get_by_hash(hash).await
        }
    }

    fn get_by_hash(&self, hash: &str) -> impl Future<Output = anyhow::Result<Option<Entity>>> + Send
    where
        Self: Sync,
    {
        async move {
            let url = self.prefix_with(&format!("{BASE_PATH}/entity/{}", hash));
            Ok(self.get_by_url(&url).await?.pop())
        }
    }

    fn find_by_slug(
        &self,
        slug: &str,
    ) -> impl Future<Output = anyhow::Result<Option<Entity>>> + Send
    where
        Self: Sync,
    {
        async move { Ok(self.find_by_index("slug", slug).await?.pop()) }
    }

    fn find_by_partial_hash(
        &self,
        hash: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send
    where
        Self: Sync,
    {
        async move {
            let mut entities = self.all().await?;
            entities.retain(|e| e.hashid().as_hex().contains(hash));

            if entities.len() > 1 {
                anyhow::bail!("duplicate entity found for hash {}", hash);
            };

            Ok(entities)
        }
    }

    fn put(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<Entity>> + Send;
    fn delete(&self, entity: &Entity) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn get_index(&self, name: &str) -> impl Future<Output = anyhow::Result<Option<Item>>> + Send;
    fn save_index_item(
        &self,
        item: &Item,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn delete_from_index(
        &self,
        entity: &Entity,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn find_by_index(
        &self,
        index: &str,
        pattern: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;
    fn all(&self) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;
    fn find(&self, pattern: &str) -> impl Future<Output = anyhow::Result<Vec<Entity>>> + Send;
    fn prefix_with(&self, path: &str) -> String;
    fn get_by_url(
        &self,
        url: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Entity>>> + Send;
}
