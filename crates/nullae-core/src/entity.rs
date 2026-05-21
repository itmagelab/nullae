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
        Format, Panel, Style,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

impl Metadata {
    fn new() -> Self {
        let created_at = chrono::Local::now().naive_local();
        Self {
            created_at,
            ..Default::default()
        }
    }

    pub fn update(&mut self) {
        let updated_at = chrono::Local::now().naive_local();
        self.updated_at = Some(updated_at);
    }

    pub fn get_attr(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.as_ref()?.get(key)
    }

    pub fn set_attr(&mut self, key: String, value: serde_json::Value) {
        let attrs = self.attributes.get_or_insert_with(std::collections::BTreeMap::new);
        attrs.insert(key, value);
    }

    pub fn remove_attr(&mut self, key: &str) -> Option<serde_json::Value> {
        let val = self.attributes.as_mut()?.remove(key);
        if let Some(attrs) = &self.attributes {
            if attrs.is_empty() {
                self.attributes = None;
            }
        }
        val
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
        let inner_str = match &self.kind {
            EntityKind::Node { inner } => format!("{}", inner),
            EntityKind::Domain { inner } => format!("{}", inner),
            EntityKind::Ip { inner } => format!("{}", inner),
            EntityKind::Url { inner } => format!("{}", inner),
        };
        write!(f, "{}", inner_str)?;

        if let Some(attrs) = self.attributes() {
            #[derive(tabled::Tabled)]
            struct AttrRow {
                #[tabled(rename = "Attribute")]
                key: String,
                #[tabled(rename = "Value")]
                value: String,
            }

            let mut rows = Vec::new();
            for (k, v) in attrs {
                let val_str = if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    serde_json::to_string(v).unwrap_or_default()
                };
                rows.push(AttrRow {
                    key: k.clone(),
                    value: val_str,
                });
            }

            let mut table = tabled::Table::new(rows);
            table.with(Style::rounded());
            table.with(Panel::header("✨  ENTITY ATTRIBUTES"));
            write!(f, "\n{}", table)?;
        }
        Ok(())
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

    pub fn attr(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get_attr(key)
    }

    pub fn set_attr(&mut self, key: String, value: serde_json::Value) {
        self.metadata.set_attr(key, value);
        self.metadata.update();
    }

    pub fn remove_attr(&mut self, key: &str) -> Option<serde_json::Value> {
        let val = self.metadata.remove_attr(key);
        self.metadata.update();
        val
    }

    pub fn attributes(&self) -> Option<&std::collections::BTreeMap<String, serde_json::Value>> {
        self.metadata.attributes.as_ref()
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

    pub async fn view<S: Storage>(vec: Vec<Self>, _ctx: &Context<S>) -> anyhow::Result<()> {
        #[derive(tabled::Tabled)]
        struct EntityRow {
            #[tabled(rename = "Hash")]
            hash: String,
            #[tabled(rename = "Type")]
            kind: String,
            #[tabled(rename = "Name / Identifier")]
            name: String,
            #[tabled(rename = "Details")]
            details: String,
        }

        let mut rows = Vec::new();

        for entity in vec {
            let hash = entity.hash().to_string();
            let kind = entity.type_name().to_string();
            let (name, details) = match &entity.kind {
                EntityKind::Node { inner } => {
                    let hostname = inner.hostname.clone();
                    let os = entity.attr("os_info")
                        .and_then(|v| serde_json::from_value::<node::OsInfo>(v.clone()).ok())
                        .map(|os| format!("{} ({})", os.os_type, os.arch))
                        .unwrap_or_default();
                    let env = inner.environment.clone().unwrap_or_default();
                    let details = if !env.is_empty() && !os.is_empty() {
                        format!("{} / {}", env, os)
                    } else if !env.is_empty() {
                        env
                    } else {
                        os
                    };
                    (hostname, details)
                }
                EntityKind::Domain { inner } => {
                    let domain_name = inner.name.clone();
                    let desc = inner.description.clone().unwrap_or_default();
                    (domain_name, desc)
                }
                EntityKind::Ip { inner } => {
                    let addr = format!("{}/{}", inner.address, inner.prefix);
                    (addr, String::new())
                }
                EntityKind::Url { inner } => {
                    let slug = inner.slug.clone();
                    let target_url = inner.url.to_string();
                    (slug, target_url)
                }
            };

            let details = if details.trim().is_empty() {
                "—".to_string()
            } else {
                details
            };

            rows.push(EntityRow {
                hash,
                kind,
                name,
                details,
            });
        }

        rows.sort_by(|a, b| match a.kind.cmp(&b.kind) {
            std::cmp::Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        });

        view(rows, "Discovered Entities");
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

    for (h, v) in headers.into_iter().zip(values) {
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
    table.with(Panel::header(header_title));

    table.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_entity_attributes_lifecycle() {
        let storage = crate::storage::memory::InMemoryStorage::new();
        let ctx = Context::with_storage(storage);

        // 1. Create a Domain entity
        let domain = domain::Domain::create("production", &ctx).await.unwrap();
        let mut entity: Entity = domain.into();

        // 2. Set different types of attributes
        entity.set_attr("env".to_string(), json!("prod"));
        entity.set_attr("priority".to_string(), json!(1));
        entity.set_attr("active".to_string(), json!(true));
        entity.set_attr("tags".to_string(), json!(["k8s", "web"]));
        entity.set_attr("nested".to_string(), json!({ "foo": "bar" }));

        // 3. Retrieve and assert attributes
        assert_eq!(entity.attr("env").unwrap(), &json!("prod"));
        assert_eq!(entity.attr("priority").unwrap(), &json!(1));
        assert_eq!(entity.attr("active").unwrap(), &json!(true));
        assert_eq!(entity.attr("tags").unwrap(), &json!(["k8s", "web"]));
        assert_eq!(entity.attr("nested").unwrap(), &json!({ "foo": "bar" }));

        // Verify BTreeMap key ordering and listing
        let attrs = entity.attributes().unwrap();
        let keys: Vec<&String> = attrs.keys().collect();
        assert_eq!(keys, vec!["active", "env", "nested", "priority", "tags"]);

        // 4. Save entity with attributes to storage
        let saved_entity = ctx.storage().save(&entity).await.unwrap();

        // 5. Get from storage and verify attributes are persisted and match
        let retrieved = ctx
            .storage()
            .get(&saved_entity.hash().as_hex())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.attr("env").unwrap(), &json!("prod"));
        assert_eq!(retrieved.attr("priority").unwrap(), &json!(1));
        assert_eq!(retrieved.attr("active").unwrap(), &json!(true));

        // 6. Modify attributes and save again
        let mut retrieved_mut = retrieved;
        retrieved_mut.set_attr("env".to_string(), json!("staging"));
        let removed = retrieved_mut.remove_attr("priority");
        assert_eq!(removed.unwrap(), json!(1));

        let saved_updated = ctx.storage().save(&retrieved_mut).await.unwrap();

        // 7. Verify modifications are stored
        let final_retrieved = ctx
            .storage()
            .get(&saved_updated.hash().as_hex())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_retrieved.attr("env").unwrap(), &json!("staging"));
        assert!(final_retrieved.attr("priority").is_none());
        assert_eq!(final_retrieved.attr("active").unwrap(), &json!(true));

        // Verify remove_attr cleans up attributes option when empty
        assert!(final_retrieved.attr("active").is_some());
        let mut final_mut = final_retrieved;
        final_mut.remove_attr("active");
        final_mut.remove_attr("env");
        final_mut.remove_attr("nested");
        final_mut.remove_attr("tags");
        assert!(final_mut.attributes().is_none());

        // Verify payload JSON serialization does not contain empty attributes field
        let payload = final_mut.payload().unwrap();
        assert!(payload.get("attributes").is_none());
    }
}
