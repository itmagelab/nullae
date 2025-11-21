use serde::{Deserialize, Serialize};
use tabled::Tabled;
use tabled::derive::display;

use crate::SHORT_HASH;
use crate::entity::EntityItem;
use crate::prelude::*;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Indexable, Entity)]
pub struct Node {
    pub(crate) hash: String,
    #[index]
    pub(crate) hostname: String,
    #[tabled(display("short_hash", self))]
    pub(crate) domain: String,
    #[tabled(display("display::option", ""))]
    pub(crate) description: Option<String>,
}

fn short_hash(hash: &str, _: &Node) -> String {
    hash[..SHORT_HASH].to_string()
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match &self.description {
            Some(d) => d.as_str(),
            None => &self.hostname,
        };
        writeln!(f, "Node ➤ {}", desc)?;
        writeln!(f, "  → Hostname: {}", self.hostname)?;
        writeln!(f, "  → Domain: {}", self.domain)?;
        writeln!(f, "  → Hash: {}", self.hash)
    }
}

impl EntityItem for Node {
    fn hash<S>(mut self, text: S) -> Self
    where
        S: Into<String>,
    {
        self.hash = text.into();
        self
    }
}

impl Node {
    pub fn new<S>(hostname: S, domain: S) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let hostname = hostname.into();
        let domain = domain.into();
        
        if hostname.trim().is_empty() {
            anyhow::bail!("Node hostname cannot be empty or whitespace-only");
        }
        
        if domain.trim().is_empty() {
            anyhow::bail!("Node domain cannot be empty or whitespace-only");
        }
        
        if hostname.len() > 255 {
            anyhow::bail!("Node hostname cannot exceed 255 characters, got: {}", hostname.len());
        }
        
        let hash = format!("{}|{}", &hostname, &domain).hash();
        Ok(Self {
            hostname,
            domain,
            hash,
            ..Default::default()
        })
    }

    pub async fn from_current_host(
        domain: &Domain,
        repository: &Repository,
    ) -> anyhow::Result<Self> {
        let hostname = hostname()?;
        Self::create(&hostname, domain, repository).await
    }

    pub async fn create(
        name: &str,
        domain: &Domain,
        repository: &Repository,
    ) -> anyhow::Result<Self> {
        let node = Self::new(name, &domain.hash)?;
        let domain = Domain::get(&domain.hash, repository).await?;
        let mut domain_entity: Entity = domain.into();
        domain_entity.add_child(&node.hash);
        repository.put(&domain_entity).await?;
        repository.create(&node.into()).await?.try_into()
    }

    pub async fn create_with_index(
        name: &str,
        domain: &Domain,
        repository: &Repository,
    ) -> anyhow::Result<Self> {
        let node = Self::create(name, domain, repository).await?;
        node.index()?.save(repository).await?;
        Ok(node)
    }

    pub async fn delete(self, repository: &Repository) -> anyhow::Result<()> {
        let Some(mut entity) = repository.find_by_hash(&self.domain).await? else {
            anyhow::bail!(
                "Can't find domain for Node: hash = {}, hostname = {}, domain = {}",
                self.hash,
                self.hostname,
                self.domain
            );
        };
        entity.remove_child(&self.hash);
        repository.put(&entity).await?;
        repository.delete(&self.into()).await?;
        Ok(())
    }
}

fn hostname() -> anyhow::Result<String> {
    let os_name = hostname::get()?;
    os_name
        .into_string()
        .map_err(|os| anyhow::anyhow!("Invalid UTF-8 in hostname: {:?}", os))
}

#[allow(dead_code)]
fn ips() -> anyhow::Result<String> {
    let interfaces = pnet::datalink::interfaces();
    let ips: Vec<std::net::IpAddr> = interfaces
        .iter()
        .flat_map(|iface| iface.ips.iter())
        .map(|ipnetwork| ipnetwork.ip())
        .collect();
    Ok(ips
        .iter()
        .map(|ip| ip.to_string())
        .collect::<Vec<_>>()
        .join("|"))
}

#[allow(dead_code)]
fn hash(str: &str) -> anyhow::Result<String> {
    let str = format!("{}|{}", str, ips()?);
    Ok(str.hash())
}
