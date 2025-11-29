use std::time::Duration;

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;

use crate::BASE_PATH;
use crate::prelude::*;
use crate::storage::Storage;

#[derive(Debug)]
pub struct Consul {
    pub(crate) url: String,
    pub(crate) client: reqwest::Client,
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
        let client = reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_nodelay(true)
            .build()?;
        Ok(Self { url, client })
    }
}

impl Storage for Consul {
    fn prefix_with(&self, path: &str) -> String {
        format!("{}/v1/kv/{}", self.url, path)
    }

    async fn put(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let payload = entity.payload()?;
        let url = self.prefix_with(&entity.path());

        self.client.put(&url).json(&payload).send().await?;

        let mut entities = self.get_by_url(&url).await?;

        if entities.is_empty() {
            anyhow::bail!("failed to get created entity");
        }

        if entities.len() > 1 {
            anyhow::bail!("multiple entities returned for single save operation");
        }

        Ok(entities.pop().unwrap())
    }

    async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        let url = self.prefix_with(&entity.path());
        self.client.delete(&url).send().await?;
        self.delete_from_index(entity).await?;
        Ok(())
    }

    async fn get_index(&self, name: &str) -> anyhow::Result<Option<Item>> {
        let url = self.prefix_with(&format!("{BASE_PATH}/index/{}", name));
        let rs = self.client.get(&url).send().await?;

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

    async fn save_index_item(&self, item: &Item) -> anyhow::Result<()> {
        let path = &item.path();
        let url = self.prefix_with(&format!("{BASE_PATH}/index/{}", path));
        let payload = item.payload()?;
        self.client.put(&url).json(&payload).send().await?;
        Ok(())
    }

    async fn delete_from_index(&self, entity: &Entity) -> anyhow::Result<()> {
        let index = Index::from_entity(entity)?;
        for new_item in index.value() {
            let Some(mut saved) = self.get_index(&new_item.path()).await? else {
                continue;
            };

            saved.subtract(new_item);
            let url = self.prefix_with(&format!("{BASE_PATH}/index/{}", &saved.path()));

            if saved.is_empty() {
                self.client.delete(&url).send().await?;
            } else {
                let payload = saved.payload()?;
                self.client.put(&url).json(&payload).send().await?;
            }
        }
        Ok(())
    }

    async fn find_by_index(&self, name: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let url = self.prefix_with(&format!("{BASE_PATH}/index/{}/{}", name, pattern));

        let rs = self.client.get(&url).send().await?;
        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        let mut records: Vec<Record> = rs.json().await?;
        let Some(record) = records.pop() else {
            return Ok(vec![]);
        };

        let item: Item = serde_json::from_value(record.value()?)?;
        let uuids: std::collections::HashSet<HashID> = item.value().into_iter().collect();

        let entities: Vec<Entity> = self
            .all()
            .await?
            .into_iter()
            .filter(|e| uuids.contains(e.hashid()))
            .collect();

        Ok(entities)
    }

    async fn all(&self) -> anyhow::Result<Vec<Entity>> {
        let url = self.prefix_with(&format!("{BASE_PATH}/entity"));

        let rs = self
            .client
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

    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let mut entities = Vec::new();
        entities.extend(self.get_by_hash(pattern).await?);
        entities.extend(self.find_by_slug(pattern).await?);
        entities.extend(self.find_by_index("hostname", pattern).await?);
        entities.extend(self.find_by_index("name", pattern).await?);
        // Very slowly
        entities.extend(self.find_by_partial_hash(pattern).await?);
        Ok(entities)
    }

    async fn get_by_url(&self, url: &str) -> anyhow::Result<Vec<Entity>> {
        let rs = self.client.get(url).send().await?;

        if !rs.status().is_success() {
            let body = rs.text().await?;
            tracing::debug!(body, "failed to get by url: {}", url);
            return Ok(vec![]);
        }

        let records: Vec<Record> = rs.json().await?;

        records
            .into_iter()
            .map(|record| record.into_entity())
            .collect()
    }
}
