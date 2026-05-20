pub mod domain;
pub mod ip;
pub mod node;
pub mod url;

use std::str::FromStr;

use crate::prelude::*;

use crate::{BASE_PATH, SHORT_HASH};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tabled::settings::object::Segment;
use tabled::{
    Tabled,
    settings::{
        Format, Style, Panel,
        object::{Columns, Object, Rows},
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

    pub fn short_hash_decorated(&self) -> String {
        format!("[{}]", self.short_hash())
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
    Node { inner: Box<Node> },
    Domain { inner: Box<Domain> },
    Ip { inner: Box<Ip> },
    Url { inner: Box<Url> },
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

        match self.kind {
            EntityKind::Node { inner } => inner.delete(ctx).await?,
            _ => {
                ctx.storage().delete(&self).await?;
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

    pub async fn view<S: Storage>(vec: Vec<Self>, ctx: &Context<S>) -> anyhow::Result<()> {
        let mut nodes = Vec::new();
        let mut domains = Vec::new();
        let mut ips = Vec::new();
        let mut urls = Vec::new();

        for entity in vec {
            match entity.kind {
                EntityKind::Node { inner } => {
                    nodes.push(NodeView::try_from_node_async(&inner, ctx).await?)
                }
                EntityKind::Domain { inner } => domains.push(DomainView::from(&*inner)),
                EntityKind::Ip { inner } => ips.push(IpView::from(&*inner)),
                EntityKind::Url { inner } => urls.push(UrlView::from(&*inner)),
            }
        }

        view(nodes, "Nodes");
        view(domains, "Domains");
        view(ips, "IPs");
        view(urls, "Urls");
        Ok(())
    }
}

fn view<T>(vec: Vec<T>, title: &str)
where
    T: Sized + Tabled,
{
    if vec.is_empty() {
        return;
    }
    println!("📦 {} (Total: {})", title.to_uppercase(), vec.len());
    let mut table = tabled::Table::new(vec);
    table.with(Style::rounded());
    table.modify(
        Columns::first(),
        Format::content(|s| {
            if s.len() > SHORT_HASH {
                s.chars().take(SHORT_HASH).collect::<String>()
            } else {
                s.to_string()
            }
        }),
    );
    table.modify(
        Segment::all().not(Rows::first()),
        Format::content(|s| s.wrap(40, None)),
    );
    println!("{table}");
    println!();
}

pub fn render_tabled_card<T: Tabled>(item: &T, header_title: &str) -> String {
    #[derive(tabled::Tabled)]
    struct KeyValueRow {
        #[tabled(rename = "Property")]
        property: String,
        #[tabled(rename = "Value")]
        value: String,
    }

    let headers = T::headers();
    let values = item.fields();
    let mut props = Vec::new();

    for (h, v) in headers.into_iter().zip(values.into_iter()) {
        let val_str = v.into_owned();
        let val_trimmed = val_str.trim();
        let display_val = if val_trimmed.is_empty() {
            "—".to_string()
        } else {
            val_str
        };
        props.push(KeyValueRow {
            property: h.into_owned(),
            value: display_val,
        });
    }

    let mut table = tabled::Table::new(props);
    table.with(Style::rounded());
    table.with(Panel::header(header_title.to_string()));
    
    table.to_string()
}
