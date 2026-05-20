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
        write!(f, "{}", crate::entity::render_tabled_card(self, "🖧  IP ADDRESS DETAILS"))
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
        ctx.storage().create(&self.into()).await?.try_into()
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

    #[test]
    fn test_ip_creation_success() {
        let parent = HashID::from_str(&"abc".hash()).unwrap();
        let ip = Ip::new("192.168.1.1", 24, &parent).unwrap();
        assert_eq!(ip.address, "192.168.1.1");
        assert_eq!(ip.prefix, 24);

        let ipv6 = Ip::new("2001:db8::1", 64, &parent).unwrap();
        assert_eq!(ipv6.address, "2001:db8::1");
        assert_eq!(ipv6.prefix, 64);
    }

    #[test]
    fn test_ip_creation_invalid_format() {
        let parent = HashID::from_str(&"abc".hash()).unwrap();
        let err = Ip::new("invalid-ip", 24, &parent).unwrap_err();
        assert!(err.to_string().contains("Invalid IP address format"));
    }

    #[test]
    fn test_ip_creation_empty() {
        let parent = HashID::from_str(&"abc".hash()).unwrap();
        let err = Ip::new("  ", 24, &parent).unwrap_err();
        assert_eq!(err.to_string(), "IP address cannot be empty or whitespace-only");
    }
}

