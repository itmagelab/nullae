use std::{net::Ipv4Addr, str::FromStr};

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Serialize, Deserialize, Debug, Clone, Indexable, Entity)]
pub struct Ip {
    pub(crate) hash: HashID,
    pub(crate) parent_hash: HashID,
    pub(crate) address: Ipv4Addr,
    pub mask: u8,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct IpView {
    pub hash: String,
    pub address: String,
}

impl From<&Ip> for IpView {
    fn from(ip: &Ip) -> Self {
        let address = format!("{}/{}", ip.address, ip.mask);
        IpView {
            hash: ip.hash.to_string(),
            address,
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
    pub fn new(address: Ipv4Addr, mask: u8, parent_hash: &HashID) -> anyhow::Result<Self> {
        let hash = HashID::from_str(&format!("{}|{}", &address, parent_hash).to_hash())?;
        Ok(Self {
            hash,
            parent_hash: parent_hash.clone(),
            address,
            mask,
        })
    }

    pub async fn get<S: Storage + Sync>(hash: &HashID, ctx: &Context<S>) -> anyhow::Result<Self> {
        let ips = ctx.storage().get_by_hash(&hash.as_hex()).await?;
        let Some(entity) = ips else {
            anyhow::bail!("Can't find IP for hash: {}", hash);
        };
        entity.try_into()
    }

    pub async fn create<S: Storage + Sync>(
        address: Ipv4Addr,
        prefix: u8,
        parent_hash: &HashID,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let ip = Self::new(address, prefix, parent_hash)?;
        if let Ok(ip) = Ip::get(&ip.hash, ctx).await {
            return Ok(ip);
        };
        ip.save(ctx).await
    }

    pub async fn save<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().create(&self.into()).await?.try_into()
    }

    pub async fn delete<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        ctx.storage().delete(&entity).await?;
        Ok(())
    }
}
