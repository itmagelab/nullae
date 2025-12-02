use std::{net::IpAddr, str::FromStr};

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::interface::Interface;
use crate::prelude::*;

/// Operating system information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OsInfo {
    pub os_type: String,
    pub arch: String,
}

/// Hardware information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_memory_gb: u64,
    pub memory_modules: Option<Vec<MemoryModule>>,
    pub storage: Option<Vec<StorageDevice>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemoryModule {
    pub label: String,
    pub size: u64,
    pub model: Option<String>,
    pub speed_mhz: Option<u64>,
    pub manufacturer: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageDevice {
    pub model: String,
    pub size: u64,
}

/// Network information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInfo {
    pub interfaces: Vec<HashID>,
}

#[derive(Default, Serialize, Deserialize, Debug, Indexable, Entity)]
pub struct Node {
    pub(crate) hash: HashID,
    #[index]
    pub(crate) hostname: String,
    pub(crate) domain: HashID,
    pub(crate) description: Option<String>,
    pub(crate) os_info: Option<OsInfo>,
    pub(crate) hardware: Option<HardwareInfo>,
    pub(crate) network: Option<NetworkInfo>,
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
    pub os_type: String,
    pub arch: String,
    pub ips: String,
    pub cpu_cores: String,
    pub total_memory_gb: String,
    pub storage: String,
    pub last_seen: String,
    pub environment: String,
    pub tags: String,
    pub description: String,
}

impl NodeView {
    pub(crate) async fn try_from_node_async<S: Storage + Sync>(
        n: &Node,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let hostname = n.hostname.clone();
        let domain = n.domain.clone();
        let domain = Domain::get(&domain, ctx).await?.name;
        let ips = n.ips(ctx).await?;

        let storage = n
            .hardware
            .as_ref()
            .and_then(|h| h.storage.as_ref())
            .map(|s| {
                let total = s.iter().map(|d| d.size).sum::<u64>() / 1024 / 1024 / 1024;
                format!("{} GB", total)
            })
            .unwrap_or_default();

        Ok(NodeView {
            hash: n.hash.to_string(),
            hostname,
            domain,
            description: n.description.clone().unwrap_or_default(),
            os_type: n
                .os_info
                .as_ref()
                .map(|os| os.os_type.clone())
                .unwrap_or_default(),
            arch: n
                .os_info
                .as_ref()
                .map(|os| os.arch.clone())
                .unwrap_or_default(),
            ips,
            cpu_cores: n
                .hardware
                .as_ref()
                .map(|hw| hw.cpu_cores.to_string())
                .unwrap_or_default(),
            total_memory_gb: n
                .hardware
                .as_ref()
                .map(|hw| hw.total_memory_gb.to_string())
                .unwrap_or_default(),
            storage,
            last_seen: n.last_seen(),
            environment: n.environment.clone().unwrap_or_default(),
            tags: n.tags(),
        })
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?
        )
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

        let hash = HashID::from_str(&format!("{}|{}", &hostname, &domain).to_hash())?;
        Ok(Self {
            hostname,
            domain,
            hash,
            ..Default::default()
        })
    }

    pub async fn from_current_host<S: Storage + Sync>(ctx: &Context<S>) -> anyhow::Result<Self> {
        let (hostname, domain) = get_host_info();
        let domain = Domain::new(domain)?;

        let domain = domain.save(ctx).await?;
        Self::create(&hostname, &domain, ctx).await
    }

    pub async fn save<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<Entity> {
        let entity: Entity = self.into();

        ctx.storage().put(&entity).await
    }

    pub async fn save_with_children<S: Storage + Sync>(
        self,
        children: Vec<HashID>,
        ctx: &Context<S>,
    ) -> anyhow::Result<Entity> {
        let mut entity: Entity = self.into();

        for hash in children {
            entity.add_child(&hash.as_hex())?;
        }

        ctx.storage().put(&entity).await
    }

    pub async fn create<S: Storage + Sync>(
        name: &str,
        domain: &Domain,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let node = Self::new(name, &domain.hash)?;
        let mut domain_entity = match ctx.storage().get_by_hash(&domain.hash.as_hex()).await? {
            Some(d) => d,
            None => {
                let domain = Domain::get(&domain.hash, ctx).await?;
                domain.into()
            }
        };
        domain_entity.add_child(&node.hash.as_hex())?;
        ctx.storage().put(&domain_entity).await?;

        let created: Self = ctx.storage().create(&node.into()).await?.try_into()?;

        created.index()?.save(ctx).await?;

        Ok(created)
    }

    pub async fn collect_host_info<S: Storage + Sync>(
        &mut self,
        ctx: &Context<S>,
    ) -> anyhow::Result<()> {
        use sysinfo::{Disks, Networks, System};

        self.os_info = Some(OsInfo {
            os_type: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        });

        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let mut unique_disks = std::collections::HashSet::new();
        let storage = disks
            .iter()
            .filter(|disk| {
                unique_disks.insert((
                    disk.name().to_string_lossy().to_string(),
                    disk.total_space(),
                    disk.available_space(),
                ))
            })
            .map(|disk| StorageDevice {
                model: disk.name().to_string_lossy().to_string(),
                size: disk.total_space(),
            })
            .collect();

        let networks = Networks::new_with_refreshed_list();
        let mut interfaces = Vec::new();

        for (name, data) in &networks {
            if name != "lo0" {
                continue;
            };
            let mut ips = Vec::new();
            let mut interface = Interface::new(name, &self.hash)?;

            for ip_net in data.ip_networks() {
                let ip_addr = ip_net.addr.to_string();
                let ip = Ip::create(&ip_addr, ip_net.prefix, &interface.hash, ctx).await?;
                ips.push(ip.hash);
            }

            interface.add_ips(ips);
            let interface = interface.save(ctx).await?;
            interfaces.push(interface.hash);
        }

        self.network = Some(NetworkInfo { interfaces });

        self.hardware = Some(HardwareInfo {
            cpu_model: sys
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .unwrap_or_else(|| "Unknown CPU".to_string()),
            cpu_cores: sys.cpus().len(),
            total_memory_gb: sys.total_memory() / 1024 / 1024 / 1024,
            memory_modules: None,
            storage: Some(storage),
        });

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

    pub async fn delete_ip<S: Storage + Sync>(
        mut self,
        hash: &HashID,
        ctx: &Context<S>,
    ) -> anyhow::Result<()> {
        if let Some(network) = &mut self.network {
            for interface in &mut network.interfaces {
                let mut interface = Interface::get(interface, ctx).await?;
                interface.ips.retain(|ip_hash| ip_hash != hash);
                interface.save(ctx).await?;
            }
        };
        ctx.storage().put(&self.into()).await?;
        Ok(())
    }

    pub async fn delete<S: Storage + Sync>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        if let Some(network) = &self.network {
            for interface in &network.interfaces {
                let interface = Interface::get(interface, ctx).await?;
                for ip_hash in &interface.ips {
                    let ip = Ip::get(ip_hash, ctx).await?;
                    ip.delete(ctx).await?;
                }
                interface.delete(ctx).await?;
            }
        }

        let Some(mut domain_entity) = ctx.storage().get_by_hash(&self.domain.as_hex()).await?
        else {
            anyhow::bail!(
                "Can't find domain for Node: hash = {}, hostname = {}, domain = {}",
                self.hash,
                self.hostname,
                self.domain
            );
        };
        domain_entity.remove_child(&self.hash.as_hex());
        ctx.storage().put(&domain_entity).await?;

        ctx.storage().delete(&self.into()).await?;

        Ok(())
    }

    pub async fn delete_batch<S: Storage + Sync>(
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
                // 1. Delete all dependent IP addresses for all nodes in parallel
                let if_deletions = domain_nodes
                    .iter()
                    .flat_map(|node| {
                        node.network
                            .as_ref()
                            .map(|net| net.interfaces.iter())
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                stream::iter(if_deletions)
                    .map(|ip_hash| async move {
                        let iface = Interface::get(ip_hash, ctx).await?;
                        iface.delete(ctx).await?;
                        Ok::<(), anyhow::Error>(())
                    })
                    .buffer_unordered(batch_size)
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<anyhow::Result<Vec<_>>>()?;

                // 2. Remove nodes from parent domain
                let Some(mut domain_entity) = ctx.storage().get_by_hash(&domain_hash).await? else {
                    anyhow::bail!("Can't find domain with hash: {}", domain_hash);
                };

                for node in &domain_nodes {
                    domain_entity.remove_child(&node.hash.as_hex());
                }

                ctx.storage().put(&domain_entity).await?;

                // 3. Delete the nodes themselves
                stream::iter(domain_nodes)
                    .map(|node| async move {
                        let node_entity: Entity = node.into();
                        ctx.storage().delete(&node_entity).await?;
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

    async fn ips<S: Storage + Sync>(&self, ctx: &Context<S>) -> anyhow::Result<String> {
        let Some(network) = &self.network else {
            return Ok(String::new());
        };

        let mut result = Vec::new();
        for interface in &network.interfaces {
            let interface = Interface::get(interface, ctx).await?;
            for h in &interface.ips {
                let ip = Ip::get(h, ctx).await?;
                result.push(ip.address);
            }
        }

        result.sort_by_key(|s| {
            let ip: IpAddr = s.parse().unwrap();
            match ip {
                IpAddr::V4(_) => 0,
                IpAddr::V6(_) => 1,
            }
        });
        Ok(result.join(" "))
    }

    fn last_seen(&self) -> String {
        self.last_seen
            .map(|x| {
                if let Some(datetime) = chrono::DateTime::from_timestamp(x, 0) {
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    x.to_string()
                }
            })
            .unwrap_or_default()
    }

    fn tags(&self) -> String {
        self.tags.as_ref().map(|v| v.join(", ")).unwrap_or_default()
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
