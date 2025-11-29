use std::collections::HashSet;

use crate::prelude::*;
use crate::{BASE_PATH, Indexable, entity::Entity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Index(Vec<Item>);

#[derive(Serialize, Deserialize, Debug)]
struct ItemData {
    key: String,
    value: Vec<HashID>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    kind: String,
    data: ItemData,
}

impl Item {
    /// Creates a new Item from references, avoiding unnecessary cloning
    pub fn new(kind: &str, key: &str, hash: &HashID) -> anyhow::Result<Self> {
        let data = ItemData {
            key: key.to_string(),
            value: vec![hash.clone()],
        };

        Ok(Self {
            kind: kind.to_string(),
            data,
        })
    }

    pub(crate) fn path(&self) -> String {
        format!("{BASE_PATH}/index/{}/{}", self.kind, self.data.key)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.value.is_empty()
    }

    fn values_mut(&mut self) -> &mut Vec<HashID> {
        &mut self.data.value
    }

    fn into_values(self) -> Vec<HashID> {
        self.data.value
    }

    fn merge(&mut self, item: Item) {
        self.values_mut().extend_from_slice(&item.into_values());
        self.values_mut().sort();
        self.values_mut().dedup();
    }

    pub(crate) fn subtract(&mut self, item: Item) {
        let set: HashSet<HashID> = item.into_values().into_iter().collect();
        self.values_mut().retain(|x| !set.contains(x));
    }

    pub(crate) fn value(self) -> Vec<HashID> {
        self.data.value
    }

    pub(crate) fn payload(&self) -> anyhow::Result<serde_json::Value> {
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
            EntityKind::Ip { inner, .. } => inner.index(),
            EntityKind::Url { inner, .. } => inner.index(),
        }
    }

    pub fn value(self) -> Vec<Item> {
        self.0
    }

    pub(crate) async fn save<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        for mut new_item in self.0 {
            if let Some(saved_item) = ctx.storage().get_index(&new_item.path()).await? {
                new_item.merge(saved_item);
            }

            ctx.storage().save_index_item(&new_item).await?;
        }

        Ok(())
    }
}
