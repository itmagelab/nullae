pub mod domain;
pub mod ip;
pub mod node;
pub mod url;

use std::str::FromStr;

use crate::prelude::*;

use crate::{BASE_PATH, SHORT_HASH};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tabled::{
    Tabled,
    settings::{
        Format, Modify, Style, Width,
        object::{Columns, Object, Rows, Segment},
        style::{HorizontalLine, LineText, VerticalLine},
    },
};

#[derive(Default, PartialEq, Clone, PartialOrd, Ord, Eq, Hash)]
pub struct HashID([u8; 32]);

impl Serialize for HashID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.as_hex())
    }
}

impl<'de> Deserialize<'de> for HashID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        HashID::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl HashID {
    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn short_hash(&self) -> String {
        self.as_hex()[..SHORT_HASH].to_string()
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for HashID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

impl std::fmt::Debug for HashID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl FromStr for HashID {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?;
        Ok(HashID(arr))
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Metadata {
    created_at: chrono::NaiveDateTime,
    updated_at: Option<chrono::NaiveDateTime>,
    children: Option<Vec<HashID>>,
}

impl Metadata {
    fn new() -> Self {
        let created_at = chrono::Local::now().naive_local();
        Self {
            created_at,
            ..Default::default()
        }
    }

    pub fn update(mut self) -> Self {
        let updated_at = chrono::Local::now().naive_local();
        self.updated_at = Some(updated_at);

        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum EntityKind {
    Node { inner: Node },
    Domain { inner: Domain },
    Ip { inner: Ip },
    Url { inner: Url },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entity {
    #[serde(flatten)]
    metadata: Metadata,
    #[serde(flatten)]
    pub kind: EntityKind,
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            EntityKind::Node { inner } => write!(f, "{}", inner),
            EntityKind::Domain { inner } => write!(f, "{}", inner),
            EntityKind::Ip { inner } => write!(f, "{}", inner),
            EntityKind::Url { inner } => write!(f, "{}", inner),
        }
    }
}

impl Entity {
    pub(crate) fn add_child(&mut self, hash: &str) -> anyhow::Result<()> {
        self.metadata
            .children
            .get_or_insert_with(Vec::new)
            .push(HashID::from_str(hash)?);
        Ok(())
    }

    fn remove_child(&mut self, hash: &str) {
        if let Some(childs) = &mut self.metadata.children {
            childs.retain(|x| x.as_hex() != hash);
            if childs.is_empty() {
                self.metadata.children = None
            };
        };
    }

    pub fn has_children(&self) -> bool {
        self.metadata.children.is_some()
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        if self.has_children() {
            anyhow::bail!("Can't delete entity with children");
        }

        // Get index before deleting entity
        let index = Index::from_entity(&self)?;

        match self.kind {
            EntityKind::Node { inner } => inner.delete(ctx).await?,
            _ => {
                // Delete entity from repository
                ctx.storage().delete(&self).await?;

                // Purge index entries
                index.purge(ctx).await?;
            }
        }
        Ok(())
    }

    pub(crate) fn payload(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    pub(crate) fn hash(&self) -> &HashID {
        match &self.kind {
            EntityKind::Node { inner } => &inner.hash,
            EntityKind::Domain { inner } => &inner.hash,
            EntityKind::Ip { inner } => &inner.hash,
            EntityKind::Url { inner } => &inner.hash,
        }
    }

    pub(crate) fn path(&self) -> String {
        format!("{BASE_PATH}/entity/{}", self.hash())
    }

    pub fn type_name(&self) -> &'static str {
        match self.kind {
            EntityKind::Node { .. } => "Node",
            EntityKind::Domain { .. } => "Domain",
            EntityKind::Ip { .. } => "Ip",
            EntityKind::Url { .. } => "Url",
        }
    }

    pub fn view(vec: Vec<Self>) {
        let mut nodes = Vec::new();
        let mut domains = Vec::new();
        let mut ips = Vec::new();
        let mut urls = Vec::new();

        for entity in vec {
            match entity.kind {
                EntityKind::Node { inner } => nodes.push(inner),
                EntityKind::Domain { inner } => domains.push(inner),
                EntityKind::Ip { inner } => ips.push(inner),
                EntityKind::Url { inner } => urls.push(inner),
            }
        }

        if !nodes.is_empty() {
            Node::view(nodes, "Node");
        }
        if !domains.is_empty() {
            Domain::view(domains, "Domain");
        }
        if !ips.is_empty() {
            Ip::view(ips, "Ip");
        }
        if !urls.is_empty() {
            Url::view(urls, "Url");
        }
    }
}

pub trait EntityItem {
    fn view(vec: Vec<Self>, title: &str)
    where
        Self: Sized + Tabled,
    {
        let mut table = tabled::Table::new(vec);
        table.with(
            Style::modern()
                .horizontals([(1, HorizontalLine::inherit(Style::modern()))])
                .verticals([(1, VerticalLine::inherit(Style::modern()))])
                .remove_horizontal(),
        );
        table.with(Modify::new(Segment::all()).with(Width::wrap(40)));
        table.with(LineText::new(title, Rows::first()).offset(1));
        table.modify(
            Columns::one(0).not(Rows::first()),
            Format::content(|s| {
                let short: String = s.chars().take(SHORT_HASH).collect();
                short
            }),
        );
        println!("{table}");
        println!();
    }
}
