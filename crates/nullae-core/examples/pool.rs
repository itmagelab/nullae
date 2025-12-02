use std::collections::{BTreeSet, HashMap};
use std::net::Ipv4Addr;

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

    /// Выдать один свободный IP
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

    /// Освободить IP
    pub fn release(&mut self, ip: &Ipv4Addr) -> bool {
        self.allocated.remove(ip)
    }

    /// Проверка, занят ли IP
    pub fn is_allocated(&self, ip: &Ipv4Addr) -> bool {
        self.allocated.contains(ip)
    }

    /// Резервировать N последовательных IP
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

    /// Резервировать конкретный IP, если он в подсети и свободен
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

/// Пул подсетей
pub struct NetworkPool {
    subnets: HashMap<String, Subnet>, // CIDR -> Subnet
}

impl NetworkPool {
    pub fn new() -> Self {
        Self {
            subnets: HashMap::new(),
        }
    }

    pub fn add_subnet(&mut self, base: Ipv4Addr, mask: u8) {
        let key = format!("{}/{}", base, mask);
        self.subnets.insert(key, Subnet::new(base, mask));
    }

    /// Выдать IP из конкретной подсети
    pub fn allocate_from(&mut self, cidr: &str) -> Option<Ipv4Addr> {
        self.subnets.get_mut(cidr)?.allocate()
    }

    /// Освободить IP
    pub fn release(&mut self, ip: &Ipv4Addr) -> bool {
        self.subnets.values_mut().any(|s| s.release(ip))
    }

    /// Проверка, занят ли IP
    pub fn is_allocated(&self, ip: &Ipv4Addr) -> bool {
        self.subnets.values().any(|s| s.is_allocated(ip))
    }

    /// Резервировать N последовательных IP из подсети
    pub fn reserve_block(&mut self, cidr: &str, count: u32) -> Option<Vec<Ipv4Addr>> {
        self.subnets.get_mut(cidr)?.reserve_block(count)
    }

    /// Резервировать конкретный IP из подсети
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

impl Default for NetworkPool {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    let mut pool = NetworkPool::new();
    pool.add_subnet("192.168.1.0".parse().unwrap(), 24);

    // Выдать первый свободный IP
    let ip1 = pool.allocate_from("192.168.1.0/24");
    println!("Allocated IP: {:?}", ip1);

    // Зарезервировать конкретный IP
    let reserved = pool.reserve_specific("192.168.1.0/24", "192.168.1.50".parse().unwrap());
    println!("Reserved 192.168.1.50: {}", reserved);

    // Попытка зарезервировать занятый IP
    let reserved_again = pool.reserve_specific("192.168.1.0/24", "192.168.1.50".parse().unwrap());
    println!("Reserved again 192.168.1.50: {}", reserved_again);

    // Резервировать блок из 3 IP
    let block = pool.reserve_block("192.168.1.0/24", 3);
    println!("Reserved block: {:?}", block);

    let ip = "192.168.1.50".parse().unwrap();
    println!("IP {} is in pool? {}", ip, pool.contains(&ip));

    let ip2 = "10.0.0.1".parse().unwrap();
    println!("IP {} is in pool? {}", ip2, pool.contains(&ip2));
}
