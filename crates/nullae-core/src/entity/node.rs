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

    // Host metrics
    #[tabled(display("display::option", ""))]
    pub(crate) os_type: Option<String>,
    #[tabled(display("display::option", ""))]
    #[index]
    pub(crate) arch: Option<String>,
    #[tabled(display("display::option", ""))]
    pub(crate) ip_address: Option<String>,
    #[tabled(display("display::option", ""))]
    pub(crate) cpu_cores: Option<usize>,
    #[tabled(display("display::option", ""))]
    pub(crate) total_memory_gb: Option<u64>,
    #[tabled(display("display::option", ""))]
    pub(crate) created_at: Option<i64>,
    #[tabled(display("display_timestamp"))]
    pub(crate) last_seen: Option<i64>,
    #[tabled(display("display::option", ""))]
    pub(crate) environment: Option<String>,
    #[tabled(display("display_tags"))]
    pub(crate) tags: Option<Vec<String>>,
}

fn display_timestamp(ts: &Option<i64>) -> String {
    match ts {
        Some(ts) => {
            if let Some(datetime) = chrono::DateTime::from_timestamp(*ts, 0) {
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                ts.to_string()
            }
        }
        None => String::new(),
    }
}

fn display_tags(tags: &Option<Vec<String>>) -> String {
    match tags {
        Some(tags) => tags.join(", "),
        None => String::new(),
    }
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
        writeln!(f, "  → Hash: {}", self.hash)?;

        // System information
        if let Some(os) = &self.os_type {
            write!(f, "  → OS: {}", os)?;
            if let Some(arch) = &self.arch {
                writeln!(f, " ({})", arch)?;
            } else {
                writeln!(f)?;
            }
        }
        if let Some(ip) = &self.ip_address {
            writeln!(f, "  → IP: {}", ip)?;
        }
        if let Some(cores) = self.cpu_cores {
            write!(f, "  → CPU: {} cores", cores)?;
            if let Some(mem) = self.total_memory_gb {
                writeln!(f, ", {} GB RAM", mem)?;
            } else {
                writeln!(f)?;
            }
        }

        // Operational information
        if let Some(env) = &self.environment {
            writeln!(f, "  → Environment: {}", env)?;
        }
        if let Some(tags) = &self.tags
            && !tags.is_empty()
        {
            writeln!(f, "  → Tags: {}", tags.join(", "))?;
        }

        Ok(())
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
            anyhow::bail!(
                "Node hostname cannot exceed 255 characters, got: {}",
                hostname.len()
            );
        }

        let hash = format!("{}|{}", &hostname, &domain).hash();
        Ok(Self {
            hostname,
            domain,
            hash,
            ..Default::default()
        })
    }

    pub async fn from_current_host(domain: &Domain, ctx: &Context) -> anyhow::Result<Self> {
        let hostname = hostname()?;
        Self::create(&hostname, domain, ctx).await
    }

    pub async fn create(name: &str, domain: &Domain, ctx: &Context) -> anyhow::Result<Self> {
        let node = Self::new(name, &domain.hash)?;
        let domain = Domain::get(&domain.hash, ctx).await?;
        let mut domain_entity: Entity = domain.into();
        domain_entity.add_child(&node.hash);
        ctx.storage().put(&domain_entity).await?;

        let created: Self = ctx.storage().create(&node.into()).await?.try_into()?;

        // Automatically save index
        created.index()?.save(ctx).await?;

        Ok(created)
    }

    /// Collects information about the current host
    pub fn collect_host_info(&mut self) -> anyhow::Result<()> {
        use sysinfo::System;

        // OS information
        self.os_type = Some(std::env::consts::OS.to_string());
        self.arch = Some(std::env::consts::ARCH.to_string());

        // IP address
        self.ip_address = local_ip_address::local_ip().ok().map(|ip| ip.to_string());

        // System information
        let mut sys = System::new_all();
        sys.refresh_all();

        self.cpu_cores = Some(sys.cpus().len());
        self.total_memory_gb = Some(sys.total_memory() / 1024 / 1024 / 1024);

        // Timestamps
        let now = chrono::Utc::now().timestamp();
        if self.created_at.is_none() {
            self.created_at = Some(now);
        }
        self.last_seen = Some(now);

        Ok(())
    }

    /// Updates only last_seen timestamp (for heartbeat)
    pub fn heartbeat(&mut self) {
        self.last_seen = Some(chrono::Utc::now().timestamp());
    }

    /// Sets the environment
    pub fn with_environment(mut self, env: &str) -> Self {
        self.environment = Some(env.to_string());
        self
    }

    /// Adds tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub async fn delete(self, ctx: &Context) -> anyhow::Result<()> {
        let Some(mut entity) = ctx.storage().find_by_hash(&self.domain).await? else {
            anyhow::bail!(
                "Can't find domain for Node: hash = {}, hostname = {}, domain = {}",
                self.hash,
                self.hostname,
                self.domain
            );
        };
        entity.remove_child(&self.hash);
        ctx.storage().put(&entity).await?;

        // Delete node entity and purge index
        let node_entity: Entity = self.into();
        let index = Index::from_entity(&node_entity)?;
        let url = ctx.storage().build_url(&node_entity.path());
        ctx.storage().pool().delete(&url).send().await?;
        index.purge(ctx).await?;

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
