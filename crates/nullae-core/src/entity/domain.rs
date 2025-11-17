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
        let desc = self.description.as_deref().unwrap_or(&self.name);
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
    pub fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        let name = name.into();
        let hash = format!("{}|", &name).hash();
        Self {
            hash,
            name,
            ..Default::default()
        }
    }

    pub async fn get(hash: &str, repository: &Repository) -> anyhow::Result<Self> {
        let domains = repository.find_by_hash(hash).await?;
        let Some(entity) = domains else {
            anyhow::bail!("Can't find domain for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create(name: &str, repository: &Repository) -> anyhow::Result<Self> {
        let domain = Self::new(name);
        if let Ok(domain) = Domain::get(&domain.hash, repository).await {
            return Ok(domain);
        };
        domain.save(repository).await
    }

    pub async fn save(self, repository: &Repository) -> anyhow::Result<Self> {
        self.index()?.save(repository).await?;
        repository.create(&self.into()).await?.try_into()
    }

    pub async fn delete(self, repository: &Repository) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(repository).await?;
        Ok(())
    }
}
