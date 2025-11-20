use std::time::Duration;

use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;

use crate::BASE_PATH;
use crate::prelude::*;

#[derive(Debug)]
pub struct Repository {
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
    pub fn value_as_slice(&self) -> anyhow::Result<Vec<u8>> {
        let value = BASE64_STANDARD.decode(&self.value)?;
        Ok(value)
    }

    pub fn value(&self) -> anyhow::Result<serde_json::Value> {
        let value = self.value_as_slice()?;
        let value: serde_json::Value = serde_json::from_slice(&value)?;
        Ok(value)
    }

    pub fn into_entity(self) -> anyhow::Result<Entity> {
        let value = self.value()?;
        serde_json::from_value(value).map_err(Into::into)
    }
}

impl Repository {
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

    async fn get_by_url<S>(&self, url: S) -> anyhow::Result<Vec<Entity>>
    where
        S: Into<String>,
    {
        let rs = self.pool.get(url.into()).send().await?;

        if !rs.status().is_success() {
            return Ok(vec![]);
        }

        let records: Vec<Record> = rs.json().await?;

        records
            .into_iter()
            .map(|record| record.into_entity())
            .collect()
    }

    pub async fn put(&self, entity: &Entity) -> anyhow::Result<Entity> {
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

    pub async fn create(&self, entity: &Entity) -> anyhow::Result<Entity> {
        if let Some(existing) = self.get(entity).await? {
            return Ok(existing);
        }

        self.put(entity).await
    }

    pub async fn get(&self, entity: &Entity) -> anyhow::Result<Option<Entity>> {
        let url = self.build_url(&entity.path());

        Ok(self.get_by_url(url).await?.pop())
    }

    pub async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        let url = self.build_url(&entity.path());
        let index = Index::from_entity(entity)?;

        self.pool.delete(&url).send().await?;

        index.purge(self).await?;
        Ok(())
    }

    pub async fn find_by_entity(&self, hash: &str) -> anyhow::Result<Vec<Entity>> {
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

    pub async fn find_by_hash(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        let url = self.build_url(&format!("{BASE_PATH}/entity/{}", hash));
        if let Some(entity) = self.get_by_url(url).await?.into_iter().next() {
            return Ok(Some(entity));
        }

        Ok(None)
    }

    pub async fn find_by_slug(&self, slug: &str) -> anyhow::Result<Option<Entity>> {
        if let Some(entity) = self.find_by_index("slug", slug).await?.pop() {
            return Ok(Some(entity));
        }

        Ok(None)
    }

    pub async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let mut vec: Vec<Entity> = vec![];

        vec.extend(self.find_by_hash(pattern).await?);
        vec.extend(self.find_by_slug(pattern).await?);

        vec.extend(self.find_by_index("hostname", pattern).await?);
        vec.extend(self.find_by_index("name", pattern).await?);

        vec.extend(self.find_by_entity(pattern).await?);
        Ok(vec)
    }

    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
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

        let mut entities = Vec::new();
        for uuid in item.value() {
            let url = self.build_url(&format!("{BASE_PATH}/entity/{}", uuid));
            let mut records: Vec<Record> = self.pool.get(&url).send().await?.json().await?;

            let Some(record) = records.pop() else {
                continue;
            };

            let entity = serde_json::from_value::<Entity>(record.value()?)?;
            entities.push(entity);
        }

        Ok(entities)
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Entity>> {
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
}
