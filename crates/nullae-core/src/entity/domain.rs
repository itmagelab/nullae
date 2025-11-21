use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use tabled::derive::display;

use crate::entity::EntityItem;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Domain {
    pub(crate) hash: String,
    #[index]
    pub(crate) name: String,
    #[tabled(display("display::option", ""))]
    pub(crate) description: Option<String>,
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

impl EntityItem for Domain {
    fn hash<S>(mut self, text: S) -> Self
    where
        S: Into<String>,
    {
        self.hash = text.into();
        self
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
            anyhow::bail!("Domain name cannot exceed 255 characters, got: {}", name.len());
        }
        
        let hash = format!("{}|", &name).hash();
        Ok(Self {
            hash,
            name,
            ..Default::default()
        })
    }

    pub async fn get(hash: &str, ctx: &Context) -> anyhow::Result<Self> {
        let domains = ctx.repository().find_by_hash(hash).await?;
        let Some(entity) = domains else {
            anyhow::bail!("Can't find domain for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create(name: &str, ctx: &Context) -> anyhow::Result<Self> {
        let domain = Self::new(name)?;
        if let Ok(domain) = Domain::get(&domain.hash, ctx).await {
            return Ok(domain);
        };
        domain.save(ctx).await
    }

    pub async fn save(self, ctx: &Context) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.repository().create(&self.into()).await?.try_into()
    }

    pub async fn delete(self, ctx: &Context) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(ctx).await?;
        Ok(())
    }
}
