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
    create_index: usize,
    flags: usize,
    key: String,
    lock_index: usize,
    modify_index: usize,
    value: String,
}

impl Record {
    pub fn create_index(&self) -> usize {
        self.create_index
    }

    pub fn flags(&self) -> usize {
        self.flags
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn lock_index(&self) -> usize {
        self.lock_index
    }

    pub fn modify_index(&self) -> usize {
        self.modify_index
    }

    pub fn value_as_slice(&self) -> anyhow::Result<Vec<u8>> {
        let value = BASE64_STANDARD.decode(&self.value)?;
        Ok(value)
    }

    pub fn value(&self) -> anyhow::Result<serde_json::Value> {
        let value = self.value_as_slice()?;
        let value: serde_json::Value = serde_json::from_slice(&value.to_vec())?;
        Ok(value)
    }
}

impl Repository {
    pub fn new() -> anyhow::Result<Self> {
        let url = std::env::var("NULLAE_CONSUL_URL").unwrap_or("http://localhost:8500".to_string());
        let pool = reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_nodelay(true)
            .build()?;
        Ok(Self { url, pool })
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
            .map(|record| {
                let value = record.value()?;
                serde_json::from_value(value).map_err(Into::into)
            })
            .collect()
    }

    pub async fn put(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let payload = entity.payload()?;
        let url = format!("{}/v1/kv/{}", self.url, entity.path());

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
        let url = format!("{}/v1/kv/{}", self.url, entity.path());

        Ok(self.get_by_url(url).await?.pop())
    }

    pub async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        let url = format!("{}/v1/kv/{}", self.url, entity.path());
        let index = Index::from_entity(entity)?;

        self.pool.delete(&url).send().await?;

        index.purge(self).await?;
        Ok(())
    }

    pub async fn find_by_entity(&self, hash: &str) -> anyhow::Result<Vec<Entity>> {
        let url = format!("{}/v1/kv/{BASE_PATH}/entity", self.url);

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
            .map(|r| {
                let value = r.value()?;
                serde_json::from_value(value).map_err(anyhow::Error::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        entities.retain(|e| e.hash().contains(hash));
        if entities.len() > 1 {
            anyhow::bail!("duplicate entity found for hash {}", hash);
        };

        Ok(entities)
    }

    pub async fn find_by_hash(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        let url = format!("{}/v1/kv/{BASE_PATH}/entity/{}", self.url, hash);
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
        let url = format!("{}/v1/kv/{BASE_PATH}/index/{}/{}", self.url, index, pattern);

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
            let url = format!("{}/v1/kv/{BASE_PATH}/entity/{}", self.url, uuid);
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
        let url = format!("{}/v1/kv/{BASE_PATH}/entity", self.url);

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
            .map(|r| serde_json::from_value(r.value()?).map_err(anyhow::Error::from))
            .collect()
    }
}
