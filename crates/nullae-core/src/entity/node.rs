use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::prelude::*;

#[derive(Default, Serialize, Deserialize, Debug, Indexable, Entity)]
pub struct Node {
    pub(crate) hash: HashID,
    #[index]
    pub(crate) hostname: String,
    pub(crate) domain: HashID,
    pub(crate) description: Option<String>,
    pub(crate) os_type: Option<String>,
    #[index]
    pub(crate) arch: Option<String>,
    pub(crate) ips: Option<Vec<HashID>>,
    pub(crate) cpu_cores: Option<usize>,
    pub(crate) total_memory_gb: Option<u64>,
    pub(crate) created_at: Option<i64>,
    pub(crate) last_seen: Option<i64>,
    pub(crate) environment: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct NodeView {
    pub hash: String,
    pub hostname: String,
    pub domain: String,
    pub description: String,
    pub os_type: String,
    pub arch: String,
    pub ips: String,
    pub cpu_cores: String,
    pub total_memory_gb: String,
    pub last_seen: String,
    pub environment: String,
    pub tags: String,
}

impl NodeView {
    pub(crate) async fn try_from_node_async<S: Storage>(
        n: &Node,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let hostname = n.hostname.clone();
        let domain = n.domain.clone();
        let domain = Domain::get(&domain, ctx).await?.name;
        Ok(NodeView {
            hash: n.hash.to_string(),
            hostname,
            domain,
            description: n.description.clone().unwrap_or_default(),
            os_type: n.os_type.clone().unwrap_or_default(),
            arch: n.arch.clone().unwrap_or_default(),
            ips: n
                .ips
                .as_ref()
                .map(|v| {
                    v.iter()
                        .map(|h| h.short_hash())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            cpu_cores: n.cpu_cores.map(|x| x.to_string()).unwrap_or_default(),
            total_memory_gb: n.total_memory_gb.map(|x| x.to_string()).unwrap_or_default(),
            last_seen: n
                .last_seen
                .map(|x| {
                    if let Some(datetime) = chrono::DateTime::from_timestamp(x, 0) {
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    } else {
                        x.to_string()
                    }
                })
                .unwrap_or_default(),
            environment: n.environment.clone().unwrap_or_default(),
            tags: n.tags.as_ref().map(|v| v.join(", ")).unwrap_or_default(),
        })
    }
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
        if let Some(ips) = &self.ips {
            writeln!(
                f,
                "  → IPs: {}",
                ips.iter()
                    .map(|h| h.as_hex())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
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

impl Node {
    pub fn new<S>(hostname: S, domain: &HashID) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let hostname = hostname.into();
        let domain = domain.clone();

        if hostname.trim().is_empty() {
            anyhow::bail!("Node hostname cannot be empty or whitespace-only");
        }

        if hostname.len() > 255 {
            anyhow::bail!(
                "Node hostname cannot exceed 255 characters, got: {}",
                hostname.len()
            );
        }

        let hash = HashID::from_str(&format!("{}|{}", &hostname, &domain).hash())?;
        Ok(Self {
            hostname,
            domain,
            hash,
            ..Default::default()
        })
    }

    pub async fn from_current_host<S: Storage>(ctx: &Context<S>) -> anyhow::Result<Self> {
        let (hostname, domain) = get_host_info();
        let domain = Domain::new(domain)?;

        let domain = domain.save(ctx).await?;
        Self::create(&hostname, &domain, ctx).await
    }

    pub async fn save<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<Entity> {
        let entity: Entity = self.into();

        ctx.storage().save(&entity).await
    }

    pub async fn save_with_children<S: Storage>(
        self,
        children: Vec<HashID>,
        ctx: &Context<S>,
    ) -> anyhow::Result<Entity> {
        let mut entity: Entity = self.into();

        for hash in children {
            entity.add_child(&hash.as_hex())?;
        }

        ctx.storage().save(&entity).await
    }

    pub async fn create<S: Storage>(
        name: &str,
        domain: &Domain,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let node = Self::new(name, &domain.hash)?;
        let mut domain_entity = match ctx.storage().get(&domain.hash.as_hex()).await? {
            Some(d) => d,
            None => {
                let domain = Domain::get(&domain.hash, ctx).await?;
                domain.into()
            }
        };
        domain_entity.add_child(&node.hash.as_hex())?;
        ctx.storage().save(&domain_entity).await?;

        let created: Self = ctx.storage().create(&node.into()).await?.try_into()?;

        created.index()?.save(ctx).await?;

        Ok(created)
    }

    pub async fn collect_host_info<S: Storage>(&mut self, _ctx: &Context<S>) -> anyhow::Result<()> {
        use sysinfo::System;

        self.os_type = Some(std::env::consts::OS.to_string());
        self.arch = Some(std::env::consts::ARCH.to_string());

        let mut sys = System::new_all();
        sys.refresh_all();

        self.cpu_cores = Some(sys.cpus().len());
        self.total_memory_gb = Some(sys.total_memory() / 1024 / 1024 / 1024);

        let now = chrono::Utc::now().timestamp();
        if self.created_at.is_none() {
            self.created_at = Some(now);
        }
        self.last_seen = Some(now);

        Ok(())
    }

    pub fn heartbeat(&mut self) {
        self.last_seen = Some(chrono::Utc::now().timestamp());
    }

    pub fn with_environment(mut self, env: &str) -> Self {
        self.environment = Some(env.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let Some(mut domain_entity) = ctx.storage().get(&self.domain.as_hex()).await? else {
            anyhow::bail!(
                "Can't find domain for Node: hash = {}, hostname = {}, domain = {}",
                self.hash,
                self.hostname,
                self.domain
            );
        };
        domain_entity.remove_child(&self.hash.as_hex());
        ctx.storage().save(&domain_entity).await?;

        let node_entity: Entity = self.into();
        let index = Index::from_entity(&node_entity)?;
        ctx.storage().delete(&node_entity).await?;
        index.purge(ctx).await?;

        Ok(())
    }

    pub async fn delete_batch<S: Storage>(
        nodes: Vec<Self>,
        ctx: &Context<S>,
        batch_size: usize,
    ) -> anyhow::Result<()> {
        use futures::stream::{self, StreamExt};
        use std::collections::HashMap;

        let mut nodes_by_domain: HashMap<String, Vec<Self>> = HashMap::new();
        for node in nodes {
            nodes_by_domain
                .entry(node.domain.as_hex())
                .or_default()
                .push(node);
        }

        stream::iter(nodes_by_domain)
            .map(|(domain_hash, domain_nodes)| async move {
                let Some(mut domain_entity) = ctx.storage().get(&domain_hash).await? else {
                    anyhow::bail!("Can't find domain with hash: {}", domain_hash);
                };

                for node in &domain_nodes {
                    domain_entity.remove_child(&node.hash.as_hex());
                }

                ctx.storage().save(&domain_entity).await?;

                stream::iter(domain_nodes)
                    .map(|node| async move {
                        let node_entity: Entity = node.into();
                        let index = Index::from_entity(&node_entity)?;
                        ctx.storage().delete(&node_entity).await?;
                        index.purge(ctx).await?;
                        Ok::<(), anyhow::Error>(())
                    })
                    .buffer_unordered(batch_size)
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<anyhow::Result<Vec<_>>>()?;

                Ok::<(), anyhow::Error>(())
            })
            .buffer_unordered(batch_size)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }
}

fn get_host_info() -> (String, String) {
    let fqdn_osstr = hostname::get().unwrap_or_default();
    let fqdn = fqdn_osstr.to_string_lossy().to_string();

    let parts: Vec<&str> = fqdn.splitn(2, '.').collect();
    let hostname = parts.first().unwrap_or(&"").to_string();
    let domain = parts.get(1).unwrap_or(&"").to_string();

    (hostname, domain)
}
