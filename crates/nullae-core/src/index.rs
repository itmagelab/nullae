use std::collections::HashSet;

use crate::prelude::*;
use crate::{BASE_PATH, Indexable, entity::Entity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Index(Vec<Item>);

#[derive(Serialize, Deserialize, Debug)]
struct ItemData {
    key: String,
    value: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    kind: String,
    data: ItemData,
}

impl Item {
    /// Creates a new Item from references, avoiding unnecessary cloning
    pub fn new(kind: &str, key: &str, hash: &str) -> anyhow::Result<Self> {
        let data = ItemData {
            key: key.to_string(),
            value: vec![hash.to_string()],
        };

        Ok(Self {
            kind: kind.to_string(),
            data,
        })
    }

    fn path(&self) -> String {
        format!("{BASE_PATH}/index/{}/{}", self.kind, self.data.key)
    }

    fn is_empty(&self) -> bool {
        self.data.value.is_empty()
    }

    fn values_mut(&mut self) -> &mut Vec<String> {
        &mut self.data.value
    }

    fn into_values(self) -> Vec<String> {
        self.data.value
    }

    fn merge(&mut self, item: Item) {
        self.values_mut().extend_from_slice(&item.into_values());
        self.values_mut().sort();
        self.values_mut().dedup();
    }

    fn subtract(&mut self, item: Item) {
        let set: HashSet<String> = item.into_values().into_iter().collect();
        self.values_mut().retain(|x| !set.contains(x));
    }

    pub(crate) fn value(self) -> Vec<String> {
        self.data.value
    }

    fn payload(self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

impl Index {
    pub(crate) fn new() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, item: Item) {
        self.0.push(item);
    }

    pub(crate) fn from_entity(entity: &Entity) -> anyhow::Result<Self> {
        match &entity.kind {
            EntityKind::Node { inner, .. } => inner.index(),
            EntityKind::Domain { inner, .. } => inner.index(),
            EntityKind::Url { inner, .. } => inner.index(),
        }
    }

    pub(crate) async fn save(self, ctx: &Context) -> anyhow::Result<()> {
        for mut new_item in self.0 {
            let url = format!("{}/v1/kv/{}", ctx.storage().url(), new_item.path());

            let Ok(rs) = ctx.storage().pool().get(&url).send().await else {
                anyhow::bail!("Got response ERROR");
            };

            let status = rs.status();

            let body = rs.text().await?;

            let mut records: Vec<Record> = if status.is_success() {
                serde_json::from_str(&body)?
            } else {
                vec![]
            };

            if let Some(record) = records.pop() {
                let saved_item: Item = serde_json::from_value(record.value()?)?;
                new_item.merge(saved_item);
            };

            ctx.storage()
                .pool()
                .put(&url)
                .json(&new_item.payload()?)
                .send()
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn purge(self, ctx: &Context) -> anyhow::Result<()> {
        for new_item in self.0 {
            let url = format!("{}/v1/kv/{}", ctx.storage().url(), new_item.path());
            let rs = ctx.storage().pool().get(&url).send().await?;

            if !rs.status().is_success() {
                continue;
            }

            let mut records: Vec<Record> = rs.json().await?;
            let Some(record) = records.pop() else {
                continue;
            };

            let mut saved: Item = serde_json::from_value(record.value()?)?;
            saved.subtract(new_item);

            if saved.is_empty() {
                ctx.storage().pool().delete(&url).send().await?;
            } else {
                let payload = saved.payload()?;
                ctx.storage()
                    .pool()
                    .put(&url)
                    .json(&payload)
                    .send()
                    .await?;
            }
        }

        Ok(())
    }
}
