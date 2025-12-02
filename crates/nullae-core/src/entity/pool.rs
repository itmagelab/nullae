use std::{
    collections::{BTreeSet, HashMap},
    net::Ipv4Addr,
    str::FromStr,
};

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Subnet {
    base: Ipv4Addr,
    mask: u8,
    allocated: BTreeSet<Ipv4Addr>,
}

impl Subnet {
    pub fn new(base: Ipv4Addr, mask: u8) -> Self {
        Self {
            base,
            mask,
            allocated: BTreeSet::new(),
        }
    }

    fn max_hosts(&self) -> u32 {
        2u32.pow((32 - self.mask) as u32) - 2
    }

    fn ip_from_index(&self, idx: u32) -> Ipv4Addr {
        let base_int: u32 = self.base.into();
        Ipv4Addr::from(base_int + idx + 1)
    }

    pub fn allocate(&mut self) -> Option<Ipv4Addr> {
        for i in 0..self.max_hosts() {
            let ip = self.ip_from_index(i);
            if !self.allocated.contains(&ip) {
                self.allocated.insert(ip);
                return Some(ip);
            }
        }
        None
    }

    pub fn release(&mut self, ip: &Ipv4Addr) -> bool {
        self.allocated.remove(ip)
    }

    pub fn is_allocated(&self, ip: &Ipv4Addr) -> bool {
        self.allocated.contains(ip)
    }

    pub fn reserve_block(&mut self, count: u32) -> Option<Vec<Ipv4Addr>> {
        for start in 0..=(self.max_hosts() - count) {
            let block: Vec<Ipv4Addr> = (0..count).map(|i| self.ip_from_index(start + i)).collect();
            if block.iter().all(|ip| !self.allocated.contains(ip)) {
                self.allocated.extend(block.iter().cloned());
                return Some(block);
            }
        }
        None
    }

    pub fn reserve_specific(&mut self, ip: Ipv4Addr) -> bool {
        let base_int: u32 = self.base.into();
        let ip_int: u32 = ip.into();
        let last_int = base_int + self.max_hosts();

        if ip_int > base_int && ip_int <= last_int && !self.allocated.contains(&ip) {
            self.allocated.insert(ip);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, ip: &Ipv4Addr) -> bool {
        let base_int: u32 = self.base.into();
        let ip_int: u32 = (*ip).into();
        let last_int = base_int + self.max_hosts();
        ip_int > base_int && ip_int <= last_int
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Indexable, Entity)]
pub struct Pool {
    pub(crate) hash: HashID,
    pub(crate) name: String,
    pub(crate) domain: HashID,
    pub(crate) subnets: HashMap<String, Subnet>,
}

impl std::fmt::Display for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IP ➤ {}", self.name)
    }
}

impl Pool {
    pub fn new<S>(name: S, domain: &HashID) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let name = name.into();
        let hash = HashID::from_str(&format!("{}|{}", &name, domain).to_hash())?;
        Ok(Self {
            hash,
            name,
            domain: domain.clone(),
            subnets: HashMap::new(),
        })
    }

    pub fn add_subnet(&mut self, base: Ipv4Addr, mask: u8) {
        let key = format!("{}/{}", base, mask);
        self.subnets.insert(key, Subnet::new(base, mask));
    }

    pub fn allocate_from(&mut self, cidr: &str) -> Option<Ipv4Addr> {
        self.subnets.get_mut(cidr)?.allocate()
    }

    pub fn release(&mut self, ip: &Ipv4Addr) -> bool {
        self.subnets.values_mut().any(|s| s.release(ip))
    }

    pub fn is_allocated(&self, ip: &Ipv4Addr) -> bool {
        self.subnets.values().any(|s| s.is_allocated(ip))
    }

    pub fn reserve_block(&mut self, cidr: &str, count: u32) -> Option<Vec<Ipv4Addr>> {
        self.subnets.get_mut(cidr)?.reserve_block(count)
    }

    pub fn reserve_specific(&mut self, cidr: &str, ip: Ipv4Addr) -> bool {
        if let Some(subnet) = self.subnets.get_mut(cidr) {
            subnet.reserve_specific(ip)
        } else {
            false
        }
    }

    pub fn contains(&self, ip: &Ipv4Addr) -> bool {
        self.subnets.values().any(|subnet| subnet.contains(ip))
    }
}
