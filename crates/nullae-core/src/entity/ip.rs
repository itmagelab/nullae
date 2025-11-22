use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::entity::EntityItem;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Ip {
    pub(crate) hash: String,
    #[index]
    pub(crate) address: String,
}

impl std::fmt::Display for Ip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IP ➤ {}", self.address)?;
        writeln!(f, "  → Hash: {}", self.hash)
    }
}

impl EntityItem for Ip {
    fn hash<S>(mut self, text: S) -> Self
    where
        S: Into<String>,
    {
        self.hash = text.into();
        self
    }
}

impl Ip {
    pub fn new<S>(address: S, parent_hash: &str) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let address = address.into();

        if address.trim().is_empty() {
            anyhow::bail!("IP address cannot be empty or whitespace-only");
        }

        // Basic validation - check if it's a valid IP address
        if address.parse::<std::net::IpAddr>().is_err() {
            anyhow::bail!("Invalid IP address format: {}", address);
        }

        let hash = format!("{}|{}", &address, parent_hash).hash();
        Ok(Self {
            hash,
            address,
            ..Default::default()
        })
    }

    pub async fn get(hash: &str, ctx: &Context) -> anyhow::Result<Self> {
        let ips = ctx.storage().find_by_hash(hash).await?;
        let Some(entity) = ips else {
            anyhow::bail!("Can't find IP for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create(address: &str, parent_hash: &str, ctx: &Context) -> anyhow::Result<Self> {
        let ip = Self::new(address, parent_hash)?;
        if let Ok(ip) = Ip::get(&ip.hash, ctx).await {
            return Ok(ip);
        };
        ip.save(ctx).await
    }

    pub async fn save(self, ctx: &Context) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn delete(self, ctx: &Context) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(ctx).await?;
        Ok(())
    }
}
