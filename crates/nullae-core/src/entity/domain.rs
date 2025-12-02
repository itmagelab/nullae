use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use tabled::derive::display;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Domain {
    pub(crate) hash: HashID,
    #[index]
    pub(crate) name: String,
    #[tabled(display("display::option", ""))]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct DomainView {
    pub(crate) hash: String,
    pub(crate) name: String,
    pub(crate) description: String,
}

impl From<&Domain> for DomainView {
    fn from(d: &Domain) -> Self {
        DomainView {
            hash: d.hash.to_string(),
            name: d.name.clone(),
            description: d.description.clone().unwrap_or_default(),
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match &self.description {
            Some(d) => d.as_str(),
            None => &self.name,
        };
        writeln!(f, "Domain ➤ {}", desc)?;
        writeln!(f, "  → Hash: {}", self.hash)?;
        writeln!(f, "  → Name: {}", self.name)
    }
}

impl Domain {
    pub fn new<S>(name: S) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let name = name.into();

        if name.trim().is_empty() {
            anyhow::bail!("Domain name cannot be empty or whitespace-only");
        }

        if name.len() > 255 {
            anyhow::bail!(
                "Domain name cannot exceed 255 characters, got: {}",
                name.len()
            );
        }

        let hash = HashID::from_str(&format!("{}|", &name).to_hash())?;
        Ok(Self {
            hash,
            name,
            ..Default::default()
        })
    }

    pub async fn entity<S: Storage + Sync>(&self, ctx: &Context<S>) -> anyhow::Result<Entity> {
        let domains = ctx.storage().get_by_hash(&self.hash.as_hex()).await?;
        let Some(entity) = domains else {
            anyhow::bail!("Can't find domain for hash: {}", self.hash);
        };
        Ok(entity)
    }

    pub async fn get<S: Storage + Sync>(hash: &HashID, ctx: &Context<S>) -> anyhow::Result<Self> {
        let domains = ctx.storage().get_by_hash(&hash.as_hex()).await?;
        let Some(entity) = domains else {
            anyhow::bail!("Can't find domain for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create<S: Storage + Sync>(name: &str, ctx: &Context<S>) -> anyhow::Result<Self> {
        let domain = Self::new(name)?;
        if let Ok(entity) = domain.entity(ctx).await {
            return entity.try_into();
        };
        domain.save(ctx).await
    }

    pub async fn save<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn delete<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.entity(ctx).await?;
        if entity.has_children() {
            anyhow::bail!(
                "Can't delete domain with children: {:?}",
                entity.metadata.children
            );
        };
        ctx.storage().delete(&entity).await?;
        Ok(())
    }
}
