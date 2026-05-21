use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Indexable, Entity)]
pub struct Ip {
    #[tabled(rename = "IP Hash")]
    pub(crate) hash: HashID,
    #[index]
    #[tabled(rename = "IP Address")]
    pub(crate) address: String,
    #[tabled(rename = "Prefix")]
    pub prefix: u8,
    #[tabled(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) attributes: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct IpView {
    pub hash: String,
    pub address: String,
}

impl From<&Ip> for IpView {
    fn from(ip: &Ip) -> Self {
        let address = format!("{}/{}", ip.address, ip.prefix);
        IpView {
            hash: ip.hash.to_string(),
            address,
        }
    }
}

impl std::fmt::Display for Ip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            crate::entity::render_tabled_card(self, "🖧  IP ADDRESS DETAILS")
        )
    }
}

impl Ip {
    pub fn new<S>(address: S, prefix: u8, parent_hash: &HashID) -> anyhow::Result<Self>
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
        Ok(Self {
            hash,
            address,
            prefix,
            ..Default::default()
        })
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

    pub async fn save<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<Self> {
        self.index()?.save(ctx).await?;
        ctx.storage().save(&self.into()).await?.try_into()
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(ctx).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ip_lifecycle_in_storage() {
        let storage = InMemoryStorage::new();
        let ctx = Context::with_storage(storage);

        // 1. Create parent Domain and Node
        let domain = Domain::create("corp", &ctx).await.unwrap();
        let node = Node::create("router-01", &domain, &ctx).await.unwrap();

        // 2. Create IP linked to that Node
        let ip = Ip::create("10.0.0.1", 8, &node.hash, &ctx).await.unwrap();
        assert_eq!(ip.address, "10.0.0.1");
        assert_eq!(ip.prefix, 8);

        // 3. Fetch from storage
        let fetched = Ip::get(&ip.hash, &ctx).await.unwrap();
        assert_eq!(fetched.address, "10.0.0.1");

        let ip_hash = ip.hash.clone();

        // 4. Delete the IP
        ip.delete(&ctx).await.unwrap();

        // 5. Verify it's gone
        let fetched_after_delete = Ip::get(&ip_hash, &ctx).await;
        assert!(fetched_after_delete.is_err());
    }
}
