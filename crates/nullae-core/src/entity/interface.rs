use std::str::FromStr;

use crate::prelude::*;
use nullae_macros::{Entity, Indexable};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone, Indexable, Entity)]
pub struct Interface {
    pub(crate) hash: HashID,
    pub name: String,
    pub ips: Vec<HashID>,
}

impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Interface ➤ {}", self.name)
    }
}

impl Interface {
    pub fn new<S>(name: S, parent_hash: &HashID) -> anyhow::Result<Self>
    where
        S: Into<String>,
    {
        let name = name.into();
        let hash = HashID::from_str(&format!("{}|{}", &name, parent_hash).to_hash())?;
        Ok(Self {
            hash,
            name,
            ..Default::default()
        })
    }

    pub fn add_ips(&mut self, ips: Vec<HashID>) {
        self.ips.extend(ips);
    }
}
