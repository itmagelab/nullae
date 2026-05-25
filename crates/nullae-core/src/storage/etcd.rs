use std::time::Duration;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};

use crate::BASE_PATH;
use crate::prelude::*;
use crate::storage::Storage;

#[derive(Debug)]
pub struct Etcd {
    pub(crate) url: String,
    pub(crate) pool: reqwest::Client,
}

#[derive(Serialize)]
struct PutRequest {
    key: String,
    value: String,
}

#[derive(Serialize)]
struct RangeRequest {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    range_end: Option<String>,
}

#[derive(Serialize)]
struct DeleteRangeRequest {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    range_end: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
struct RangeResponse {
    #[serde(default)]
    kvs: Vec<KeyValue>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct KeyValue {
    key: String,
    value: String,
}

impl KeyValue {
    pub fn value_json(&self) -> anyhow::Result<serde_json::Value> {
        let bytes = BASE64_STANDARD.decode(&self.value)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn into_entity(self) -> anyhow::Result<Entity> {
        serde_json::from_value(self.value_json()?).map_err(Into::into)
    }
}

impl Etcd {
    pub fn new() -> anyhow::Result<Self> {
        let url = std::env::var("NULLAE_ETCD_URL")
            .map_err(|_| anyhow::anyhow!("NULLAE_ETCD_URL environment variable is required"))?;
        let pool = reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_nodelay(true)
            .build()?;
        Ok(Self { url, pool })
    }

    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.url, endpoint)
    }

    fn encode_base64<T: AsRef<[u8]>>(data: T) -> String {
        BASE64_STANDARD.encode(data)
    }

    fn range_end_for_prefix(prefix: &str) -> Option<String> {
        let mut bytes = prefix.as_bytes().to_vec();
        while let Some(last) = bytes.last_mut() {
            if *last < 0xff {
                *last += 1;
                return Some(Self::encode_base64(bytes));
            } else {
                bytes.pop();
            }
        }
        None
    }

    async fn get_kvs_by_url(&self, key: &str, is_prefix: bool) -> anyhow::Result<Vec<KeyValue>> {
        let url = self.build_url("/v3/kv/range");
        let b64_key = Self::encode_base64(key);
        let range_end = if is_prefix {
            Self::range_end_for_prefix(key)
        } else {
            None
        };

        let req = RangeRequest {
            key: b64_key,
            range_end,
        };

        let rs = self.pool.post(&url).json(&req).send().await?;
        if !rs.status().is_success() {
            let status = rs.status();
            let body = rs.text().await.unwrap_or_default();
            anyhow::bail!("etcd range request failed with status {}: {}", status, body);
        }

        let resp: RangeResponse = rs.json().await?;
        Ok(resp.kvs)
    }

    async fn get_by_key(&self, key: &str) -> anyhow::Result<Vec<Entity>> {
        let kvs = self.get_kvs_by_url(key, false).await?;
        kvs.into_iter()
            .map(|kv| kv.into_entity())
            .collect()
    }

    async fn get_prefix(&self, prefix: &str) -> anyhow::Result<Vec<Entity>> {
        let kvs = self.get_kvs_by_url(prefix, true).await?;
        kvs.into_iter()
            .map(|kv| kv.into_entity())
            .collect()
    }

    async fn find_by_partial_hash(&self, hash: &str) -> anyhow::Result<Vec<Entity>> {
        let prefix = format!("{BASE_PATH}/entity/");
        let mut entities = self.get_prefix(&prefix).await?;
        entities.retain(|e| e.hash().as_hex().contains(hash));
        if entities.len() > 1 {
            anyhow::bail!("duplicate entity found for hash {}", hash);
        }
        Ok(entities)
    }

    async fn find_by_slug(&self, slug: &str) -> anyhow::Result<Option<Entity>> {
        Ok(self.find_by_index("slug", slug).await?.pop())
    }

    async fn get_entity(&self, entity: &Entity) -> anyhow::Result<Option<Entity>> {
        let key = entity.path();
        Ok(self.get_by_key(&key).await?.pop())
    }
}

impl Storage for Etcd {
    async fn get_index(&self, path: &str) -> anyhow::Result<Option<Item>> {
        let kvs = self.get_kvs_by_url(path, false).await?;
        if let Some(kv) = kvs.into_iter().next() {
            let bytes = BASE64_STANDARD.decode(&kv.value)?;
            let item: Item = serde_json::from_slice(&bytes)?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, entity: &Entity) -> anyhow::Result<Entity> {
        let key = entity.path();
        let payload = entity.payload()?;
        let value_str = serde_json::to_string(&payload)?;

        let put_url = self.build_url("/v3/kv/put");
        let req = PutRequest {
            key: Self::encode_base64(&key),
            value: Self::encode_base64(value_str),
        };

        let rs = self.pool.post(&put_url).json(&req).send().await?;
        if !rs.status().is_success() {
            let status = rs.status();
            let body = rs.text().await.unwrap_or_default();
            anyhow::bail!("etcd put failed with status {}: {}", status, body);
        }

        let entity = self
            .get_by_key(&key)
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("failed to get created entity"))?;

        Ok(entity)
    }

    async fn save_index(&self, path: &str, item: &Item) -> anyhow::Result<()> {
        let payload = item.payload()?;
        let value_str = serde_json::to_string(&payload)?;

        let put_url = self.build_url("/v3/kv/put");
        let req = PutRequest {
            key: Self::encode_base64(path),
            value: Self::encode_base64(value_str),
        };

        let rs = self.pool.post(&put_url).json(&req).send().await?;
        if !rs.status().is_success() {
            let status = rs.status();
            let body = rs.text().await.unwrap_or_default();
            anyhow::bail!("etcd put failed with status {}: {}", status, body);
        }
        Ok(())
    }

    async fn create(&self, entity: &Entity) -> anyhow::Result<Entity> {
        if let Some(existing) = self.get_entity(entity).await? {
            return Ok(existing);
        }
        self.save(entity).await
    }

    async fn get(&self, hash: &str) -> anyhow::Result<Option<Entity>> {
        let key = format!("{BASE_PATH}/entity/{}", hash);
        Ok(self.get_by_key(&key).await?.pop())
    }

    async fn delete_index(&self, path: &str) -> anyhow::Result<()> {
        let del_url = self.build_url("/v3/kv/deleterange");
        let req = DeleteRangeRequest {
            key: Self::encode_base64(path),
            range_end: None,
        };

        let rs = self.pool.post(&del_url).json(&req).send().await?;
        if !rs.status().is_success() {
            let status = rs.status();
            let body = rs.text().await.unwrap_or_default();
            anyhow::bail!("etcd deleterange failed with status {}: {}", status, body);
        }
        Ok(())
    }

    async fn delete(&self, entity: &Entity) -> anyhow::Result<()> {
        let key = entity.path();
        let del_url = self.build_url("/v3/kv/deleterange");
        let req = DeleteRangeRequest {
            key: Self::encode_base64(key),
            range_end: None,
        };

        let rs = self.pool.post(&del_url).json(&req).send().await?;
        if !rs.status().is_success() {
            let status = rs.status();
            let body = rs.text().await.unwrap_or_default();
            anyhow::bail!("etcd deleterange failed with status {}: {}", status, body);
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
        let prefix = format!("{BASE_PATH}/entity/");
        self.get_prefix(&prefix).await
    }

    async fn find_by_index(&self, index: &str, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        // 1. Get index with UUIDs
        let path = format!("{BASE_PATH}/index/{}/{}", index, pattern);
        let Some(item) = self.get_index(&path).await? else {
            return Ok(vec![]);
        };
        let uuids: std::collections::HashSet<HashID> = item.value().into_iter().collect();

        // 2. Fetch all entities in one batch request
        let prefix = format!("{BASE_PATH}/entity/");
        let entities = self.get_prefix(&prefix).await?;

        // 3. Filter entities by UUIDs from index
        let filtered = entities
            .into_iter()
            .filter(|e| uuids.contains(e.hash()))
            .collect();

        Ok(filtered)
    }

    async fn find(&self, pattern: &str) -> anyhow::Result<Vec<Entity>> {
        let mut entities = Vec::new();
        if let Some(e) = self.get(pattern).await? {
            entities.push(e);
        }
        entities.extend(self.find_by_slug(pattern).await?);
        entities.extend(self.find_by_index("hostname", pattern).await?);
        entities.extend(self.find_by_index("name", pattern).await?);
        entities.extend(self.find_by_partial_hash(pattern).await?);
        Ok(entities)
    }
}
