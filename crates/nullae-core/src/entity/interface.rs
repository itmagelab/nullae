use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone, Indexable, Entity)]
pub struct Interface {
    pub(crate) hash: HashID,
    pub(crate) parent_hash: HashID,
    pub name: String,
    pub ips: Vec<HashID>,
}

impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Interface ➤ {}", self.name)
    }
}

impl Interface {
    pub fn new<S>(name: S, parent_hash: &HashID) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let name = name.into();
        let hash = HashID::from_str(&format!("{}|{}", &name, parent_hash).to_hash())?;
        Ok(Self {
            hash,
            parent_hash: parent_hash.clone(),
            name,
            ..Default::default()
        })
    }

    pub fn add_ips(&mut self, ips: Vec<HashID>) {
        self.ips.extend(ips);
    }

    pub async fn save<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn get<S: Storage + Sync>(hash: &HashID, ctx: &Context<S>) -> anyhow::Result<Self> {
        let ips = ctx.storage().get_by_hash(&hash.as_hex()).await?;
        let Some(entity) = ips else {
            anyhow::bail!("Can't find entity for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn delete<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        ctx.storage().delete(&entity).await?;
        Ok(())
    }
}
