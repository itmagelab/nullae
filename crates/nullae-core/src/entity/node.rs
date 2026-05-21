use std::{net::IpAddr, str::FromStr};

use serde::{Deserialize, Serialize};
use tabled::Tabled;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: Option<String>,
    pub ips: Vec<HashID>,
}

/// Network information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
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
    #[tabled(rename = "Hash")]
    pub hash: String,
    #[tabled(rename = "Hostname")]
    pub hostname: String,
    #[tabled(rename = "Domain")]
    pub domain: String,
    #[tabled(rename = "OS (Arch)")]
    pub os: String,
    #[tabled(rename = "Specs (CPU/RAM/Disk)")]
    pub specs: String,
    #[tabled(rename = "IP Addresses")]
    pub ips: String,
    #[tabled(rename = "Env")]
    pub environment: String,
    #[tabled(rename = "Tags")]
    pub tags: String,
}

#[allow(dead_code)]
impl NodeView {
    pub(crate) async fn try_from_node_async<S: Storage>(
        n: &Node,
        ctx: &Context<S>,
    ) -> anyhow::Result<Self> {
        let hostname = n.hostname.clone();
        let domain = n.domain.clone();
        let domain = Domain::get(&domain, ctx).await?.name;

        let mut raw_ips = Vec::new();
        if let Some(network) = &n.network {
            for interface in &network.interfaces {
                for h in &interface.ips {
                    let ip = Ip::get(h, ctx).await?;
                    raw_ips.push(ip.address);
                }
            }
        }
        raw_ips.sort_by_key(|s| {
            let ip: std::net::IpAddr = s.parse().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
            match ip {
                std::net::IpAddr::V4(_) => 0,
                std::net::IpAddr::V6(_) => 1,
            }
        });

        let ips = if raw_ips.is_empty() {
            "—".to_string()
        } else if raw_ips.len() > 3 {
            format!("{}, +{} more", raw_ips[..3].join(", "), raw_ips.len() - 3)
        } else {
            raw_ips.join(", ")
        };

        let os = n
            .os_info
            .as_ref()
            .map(|os| format!("{} ({})", os.os_type, os.arch))
            .unwrap_or_else(|| "—".to_string());

        let specs = n
            .hardware
            .as_ref()
            .map(|hw| {
                let storage = hw
                    .storage
                    .as_ref()
                    .map(|s| {
                        let total = s.iter().map(|d| d.size).sum::<u64>() / 1024 / 1024 / 1024;
                        format!("{}GB Disk", total)
                    })
                    .unwrap_or_else(|| "—".to_string());
                format!(
                    "{}c / {}GB RAM / {}",
                    hw.cpu_cores, hw.total_memory_gb, storage
                )
            })
            .unwrap_or_else(|| "—".to_string());

        let tags = n.tags();
        let tags = if tags.is_empty() {
            "—".to_string()
        } else {
            tags
        };

        Ok(NodeView {
            hash: n.hash.to_string(),
            hostname,
            domain,
            os,
            specs,
            ips,
            environment: n.environment.clone().unwrap_or_else(|| "—".to_string()),
            tags,
        })
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(tabled::Tabled)]
        struct NodeProperty {
            #[tabled(rename = "System Property")]
            property: String,
            #[tabled(rename = "Value")]
            value: String,
        }

        let mut props = Vec::new();
        props.push(NodeProperty {
            property: "Hostname".to_string(),
            value: self.hostname.clone(),
        });
        props.push(NodeProperty {
            property: "Domain".to_string(),
            value: self.domain.short_hash_decorated(),
        });
        props.push(NodeProperty {
            property: "Node Hash".to_string(),
            value: self.hash.as_hex(),
        });

        if let Some(os_info) = &self.os_info {
            props.push(NodeProperty {
                property: "OS".to_string(),
                value: format!("{} ({})", os_info.os_type, os_info.arch),
            });
        }

        if let Some(hardware) = &self.hardware {
            props.push(NodeProperty {
                property: "CPU".to_string(),
                value: format!("{} ({} cores)", hardware.cpu_model, hardware.cpu_cores),
            });
            props.push(NodeProperty {
                property: "RAM".to_string(),
                value: format!("{} GB RAM", hardware.total_memory_gb),
            });

            if let Some(storage) = &hardware.storage {
                for disk in storage {
                    props.push(NodeProperty {
                        property: "Storage".to_string(),
                        value: format!(
                            "{} ({:.1} GB)",
                            disk.model,
                            disk.size as f64 / 1024.0 / 1024.0 / 1024.0
                        ),
                    });
                }
            }
        }

        if let Some(env) = &self.environment {
            props.push(NodeProperty {
                property: "Environment".to_string(),
                value: env.clone(),
            });
        }

        if let Some(tags) = &self.tags
            && !tags.is_empty()
        {
            props.push(NodeProperty {
                property: "Tags".to_string(),
                value: tags.join(", "),
            });
        }

        let mut prop_table = tabled::Table::new(props);
        prop_table.with(tabled::settings::Style::rounded());

        let desc = match &self.description {
            Some(d) => d.as_str(),
            None => &self.hostname,
        };
        let header_title = format!("🖳  NODE CARD: {}", desc);
        prop_table.with(tabled::settings::Panel::header(header_title));

        writeln!(f, "{}", prop_table)?;

        // Network interfaces table
        if let Some(network) = &self.network
            && !network.interfaces.is_empty()
        {
            #[derive(tabled::Tabled)]
            struct NetIface {
                #[tabled(rename = "Interface")]
                interface: String,
                #[tabled(rename = "MAC Address")]
                mac: String,
                #[tabled(rename = "IP Addresses (Hash)")]
                ips: String,
            }

            let mut active_ifaces = Vec::new();
            let mut inactive_names = Vec::new();

            for interface in &network.interfaces {
                let has_ips = !interface.ips.is_empty();
                let has_real_mac = interface.mac.as_deref().unwrap_or("00:00:00:00:00:00")
                    != "00:00:00:00:00:00"
                    && !interface.mac.as_deref().unwrap_or("").is_empty();

                if has_ips || has_real_mac {
                    let mac = interface.mac.as_deref().unwrap_or("No MAC").to_string();
                    let ips_str = if interface.ips.is_empty() {
                        "—".to_string()
                    } else {
                        interface
                            .ips
                            .iter()
                            .map(|h| h.short_hash_decorated())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    active_ifaces.push(NetIface {
                        interface: interface.name.clone(),
                        mac,
                        ips: ips_str,
                    });
                } else {
                    inactive_names.push(interface.name.clone());
                }
            }

            if !active_ifaces.is_empty() {
                let mut net_table = tabled::Table::new(active_ifaces);
                net_table.with(tabled::settings::Style::rounded());
                net_table.with(tabled::settings::Panel::header(
                    "🖧  NETWORK INTERFACES (Active)",
                ));
                writeln!(f, "{}", net_table)?;
            }

            if !inactive_names.is_empty() {
                inactive_names.sort();
                writeln!(f, "  Inactive/Virtual: {}", inactive_names.join(", "))?;
                writeln!(f)?;
            }
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

    pub async fn collect_host_info<S: Storage>(&mut self, ctx: &Context<S>) -> anyhow::Result<()> {
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
            let mut ips = Vec::new();
            for ip_net in data.ip_networks() {
                let ip_addr = ip_net.addr.to_string();
                let ip_entity = Ip::create(&ip_addr, ip_net.prefix, &self.hash, ctx).await?;
                ips.push(ip_entity.hash);
            }

            interfaces.push(NetworkInterface {
                name: name.clone(),
                mac: Some(data.mac_address().to_string()),
                ips,
            });
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
            memory_modules: None, // Not supported by sysinfo automatically
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
        ctx.storage().delete(&node_entity).await?;

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

    #[allow(dead_code)]
    pub(crate) async fn ips<S: Storage>(&self, ctx: &Context<S>) -> anyhow::Result<String> {
        let Some(network) = &self.network else {
            return Ok(String::new());
        };

        let mut result = Vec::new();
        for interface in &network.interfaces {
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

    #[allow(dead_code)]
    pub(crate) fn last_seen(&self) -> String {
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

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_lifecycle_in_storage() {
        let storage = InMemoryStorage::new();
        let ctx = Context::with_storage(storage);

        // 1. Create Domain
        let domain = Domain::create("production", &ctx).await.unwrap();

        // 2. Create Node under that Domain
        let node = Node::create("app-server-01", &domain, &ctx).await.unwrap();
        assert_eq!(node.hostname, "app-server-01");
        assert_eq!(node.domain, domain.hash);

        // 3. Verify Domain has Node in its children
        let domain_entity = ctx
            .storage()
            .get(&domain.hash.as_hex())
            .await
            .unwrap()
            .unwrap();
        assert!(domain_entity.has_children());

        // 4. Find the Node by hostname and partial hash
        let found = ctx.storage().find("app-server-01").await.unwrap();
        assert!(!found.is_empty());
        assert_eq!(found[0].hash(), &node.hash);

        let node_hash = node.hash.clone();

        // 5. Delete the Node
        node.delete(&ctx).await.unwrap();

        // 6. Verify Node is removed and Domain no longer lists it as a child
        let fetched_node_after_delete = ctx.storage().get(&node_hash.as_hex()).await.unwrap();
        assert!(fetched_node_after_delete.is_none());

        let domain_entity_after = ctx
            .storage()
            .get(&domain.hash.as_hex())
            .await
            .unwrap()
            .unwrap();
        assert!(!domain_entity_after.has_children());
    }
}
