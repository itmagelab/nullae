use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use tabled::derive::display;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Domain {
    #[tabled(rename = "Domain Hash")]
    pub(crate) hash: HashID,
    #[index]
    #[tabled(rename = "Domain Name")]
    pub(crate) name: String,
    #[tabled(rename = "Description", display("display::option", ""))]
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
        write!(
            f,
            "{}",
            crate::entity::render_tabled_card(self, "🖽  DOMAIN DETAILS")
        )
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

        let hash = HashID::from_str(&format!("{}|", &name).hash())?;
        Ok(Self {
            hash,
            name,
            ..Default::default()
        })
    }

    pub async fn get<S: Storage>(hash: &HashID, ctx: &Context<S>) -> anyhow::Result<Self> {
        let domains = ctx.storage().get(&hash.as_hex()).await?;
        let Some(entity) = domains else {
            anyhow::bail!("Can't find domain for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create<S: Storage>(name: &str, ctx: &Context<S>) -> anyhow::Result<Self> {
        let domain = Self::new(name)?;
        if let Ok(domain) = Domain::get(&domain.hash, ctx).await {
            return Ok(domain);
        };
        domain.save(ctx).await
    }

    pub async fn save<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity = match ctx.storage().get(&self.hash.as_hex()).await? {
            Some(d) => d,
            None => {
                let domain = Domain::get(&self.hash, ctx).await?;
                domain.into()
            }
        };
        entity.delete(ctx).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_creation_success() {
        let domain = Domain::new("example.com").unwrap();
        assert_eq!(domain.name, "example.com");
        assert!(domain.description.is_none());
        assert!(!domain.hash.as_hex().is_empty());
    }

    #[test]
    fn test_domain_creation_empty_err() {
        let err = Domain::new("   ").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Domain name cannot be empty or whitespace-only"
        );
    }

    #[test]
    fn test_domain_creation_too_long_err() {
        let name = "a".repeat(256);
        let err = Domain::new(name).unwrap_err();
        assert!(
            err.to_string()
                .contains("Domain name cannot exceed 255 characters")
        );
    }
}
