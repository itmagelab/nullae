use std::time::Duration;

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;

use crate::BASE_PATH;
use crate::prelude::*;
use crate::storage::Storage;

#[derive(Debug)]
pub struct Consul {
    pub(crate) url: String,
    pub(crate) pool: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Record {
    pub create_index: usize,
    pub flags: usize,
    pub key: String,
    pub lock_index: usize,
    pub modify_index: usize,
    value: String,
}

impl Record {
    pub fn value(&self) -> anyhow::Result<serde_json::Value> {
        let bytes = BASE64_STANDARD.decode(&self.value)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn into_entity(self) -> anyhow::Result<Entity> {
        serde_json::from_value(self.value()?).map_err(Into::into)
    }
}

impl Consul {
    pub fn new() -> anyhow::Result<Self> {
        let url = std::env::var("NULLAE_CONSUL_URL")
            .map_err(|_| anyhow::anyhow!("NULLAE_CONSUL_URL environment variable is required"))?;
        let pool = reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_nodelay(true)
            .build()?;
        Ok(Self { url, pool })
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}/v1/kv/{}", self.url, path)
    }

    async fn get_by_url(&self, url: &str) -> anyhow::Result<Vec<Entity>> {
        let rs = self.pool.get(url).send().await?;

        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        let records: Vec<Record> = rs.json().await?;

        records
            .into_iter()
            .map(|record| record.into_entity())
            .collect()
    }

    async fn find_by_partial_hash(&self, hash: &str) -> anyhow::Result<Vec<Entity>> {
        let url = self.build_url(&format!("{BASE_PATH}/entity"));

        let rs = self
            .pool
            .get(&url)
            .query(&serde_json::json!({ "recurse": true }))
            .send()
            .await?;

        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        let mut entities: Vec<Entity> = rs
            .json::<Vec<Record>>()
            .await?
            .into_iter()
            .map(|r| r.into_entity())
            .collect::<Result<Vec<_>, _>>()?;
        entities.retain(|e| e.hash().contains(hash));
        if entities.len() > 1 {
            anyhow::bail!("duplicate entity found for hash {}", hash);
        };

        Ok(entities)
    }

    async fn find_by_slug(&self, slug: &str) -> anyhow::Result<Option<Entity>> {
        Ok(self.find_by_index("slug", slug).await?.pop())
    }

    async fn get_entity(&self, entity: &Entity) -> anyhow::Result<Option<Entity>> {
        let url = self.build_url(&entity.path());
        Ok(self.get_by_url(&url).await?.pop())
    }
}

#[async_trait::async_trait]
impl Storage for Consul {
    async fn get_index(&self, path: &str) -> anyhow::Result<Option<Item>> {
        let url = self.build_url(&format!("{BASE_PATH}/index/{}", path));
        let rs = self.pool.get(&url).send().await?;

        if !rs.status().is_success() {
            return Ok(None);
        }

        let mut records: Vec<Record> = rs.json().await?;
        if let Some(record) = records.pop() {
            let item: Item = serde_json::from_value(record.value()?)?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let payload = entity.payload()?;
        let url = self.build_url(&entity.path());

        self.pool.put(&url).json(&payload).send().await?;

        let entity = self
            .get_by_url(&url)
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("failed to get created entity"))?;

        Ok(entity)
    }

    async fn save_index(&self, path: &str, item: &Item) -> anyhow::Result<()> {
        let url = self.build_url(&format!("{BASE_PATH}/index/{}", path));
        let payload = item.payload()?;
        self.pool.put(&url).json(&payload).send().await?;
        Ok(())
    }

    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity> {
        if let Some(existing) = self.get_entity(entity).await? {
            return Ok(existing);
        }

        self.save(entity).await
    }

    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let mut entities = Vec::new();
        entities.extend(self.get(pattern).await?);
        entities.extend(self.find_by_slug(pattern).await?);
        entities.extend(self.find_by_index("hostname", pattern).await?);
        entities.extend(self.find_by_index("name", pattern).await?);
        entities.extend(self.find_by_partial_hash(pattern).await?);
        Ok(entities)
    }

    async fn get(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        let url = self.build_url(&format!("{BASE_PATH}/entity/{}", hash));
        Ok(self.get_by_url(&url).await?.pop())
    }

    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        // 1. Get index with UUIDs
        let url = self.build_url(&format!("{BASE_PATH}/index/{}/{}", index, pattern));

        let rs = self.pool.get(&url).send().await?;
        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        let mut records: Vec<Record> = rs.json().await?;
        let Some(record) = records.pop() else {
            return Ok(vec![]);
        };

        let item: Item = serde_json::from_value(record.value()?)?;
        let uuids: std::collections::HashSet<String> = item.value().into_iter().collect();

        // 2. Fetch all entities in one batch request
        let url = self.build_url(&format!("{BASE_PATH}/entity"));
        let rs = self
            .pool
            .get(&url)
            .query(&serde_json::json!({"recurse": true}))
            .send()
            .await?;

        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        // 3. Filter entities by UUIDs from index
        let entities: Vec<Entity> = rs
            .json::<Vec<Record>>()
            .await?
            .into_iter()
            .filter_map(|r| {
                r.into_entity()
                    .ok()
                    .filter(|e| uuids.contains(e.hash() as &str))
            })
            .collect();

        Ok(entities)
    }

    async fn list(&self) -> anyhow::Result<Vec<Entity>> {
        let url = self.build_url(&format!("{BASE_PATH}/entity"));

        let rs = self
            .pool
            .get(&url)
            .query(&serde_json::json!({ "recurse": true }))
            .send()
            .await?;

        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        rs.json::<Vec<Record>>()
            .await?
            .into_iter()
            .map(|r| r.into_entity())
            .collect()
    }

    async fn delete_index(&self, path: &str) -> anyhow::Result<()> {
        let url = self.build_url(&format!("{BASE_PATH}/index/{}", path));
        self.pool.delete(&url).send().await?;
        Ok(())
    }

    async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        let url = self.build_url(&entity.path());
        self.pool.delete(&url).send().await?;
        Ok(())
    }
}
