use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Ip {
    pub(crate) hash: HashID,
    #[index]
    pub(crate) address: String,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct IpView {
    pub hash: String,
    pub address: String,
}

impl From<&Ip> for IpView {
    fn from(ip: &Ip) -> Self {
        IpView {
            hash: ip.hash.to_string(),
            address: ip.address.clone(),
        }
    }
}

impl std::fmt::Display for Ip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IP ➤ {}", self.address)?;
        writeln!(f, "  → Hash: {}", self.hash)
    }
}

impl Ip {
    pub fn new<S>(address: S, parent_hash: &HashID) -> anyhow::Result<Self>
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

        let hash = HashID::from_str(&format!("{}|{}", &address, parent_hash).hash())?;
        Ok(Self { hash, address })
    }

    pub async fn get<S: Storage>(hash: &HashID, ctx: &Context<S>) -> anyhow::Result<Self> {
        let ips = ctx.storage().get(&hash.as_hex()).await?;
        let Some(entity) = ips else {
            anyhow::bail!("Can't find IP for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create<S: Storage>(
        address: &str,
        parent_hash: &HashID,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let ip = Self::new(address, parent_hash)?;
        if let Ok(ip) = Ip::get(&ip.hash, ctx).await {
            return Ok(ip);
        };
        ip.save(ctx).await
    }

    pub async fn save<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(ctx).await?;
        Ok(())
    }
}
