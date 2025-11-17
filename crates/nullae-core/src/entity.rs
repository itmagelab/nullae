pub mod domain;
pub mod node;
pub mod url;

use crate::prelude::*;
use std::collections::HashMap;

use crate::{BASE_PATH, SHORT_HASH};
use serde::{Deserialize, Serialize};
use tabled::{
    Tabled,
    settings::{
        Format, Style,
        object::{Columns, Object, Rows},
        style::{HorizontalLine, LineText, VerticalLine},
    },
};

use crate::Metadata;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum EntityKind {
    Node { inner: Node },
    Domain { inner: Domain },
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
            EntityKind::Node { inner, .. } => write!(f, "{}", inner),
            EntityKind::Domain { inner, .. } => write!(f, "{}", inner),
            EntityKind::Url { inner, .. } => write!(f, "{}", inner),
        }
    }
}

impl Entity {
    fn add_child(&mut self, hash: &str) {
        self.metadata
            .children
            .get_or_insert_with(Vec::new)
            .push(hash.to_owned());
    }

    fn remove_child(&mut self, hash: &str) {
        if let Some(childs) = &mut self.metadata.children {
            childs.retain(|x| x != hash);
            if childs.is_empty() {
                self.metadata.children = None
            };
        };
    }

    pub fn has_chindren(&self) -> bool {
        self.metadata.children.is_some()
    }

    pub async fn delete(self, repository: &Repository) -> anyhow::Result<()> {
        if self.has_chindren() {
            anyhow::bail!("Can't delete entity with children");
        };
        match self.kind {
            EntityKind::Node { inner, .. } => inner.delete(repository).await?,
            _ => repository.delete(&self).await?,
        };
        Ok(())
    }

    pub(crate) fn payload(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    pub(crate) fn hash(&self) -> String {
        match &self.kind {
            EntityKind::Node { inner, .. } => inner.hash.clone(),
            EntityKind::Domain { inner, .. } => inner.hash.clone(),
            EntityKind::Url { inner, .. } => inner.hash.clone(),
        }
    }

    pub(crate) fn path(&self) -> String {
        let id = self.hash();
        let path = format!("{BASE_PATH}/entity");
        format!("{path}/{id}")
    }

    pub fn type_name(&self) -> &'static str {
        match self.kind {
            EntityKind::Node { .. } => "Node",
            EntityKind::Domain { .. } => "Domain",
            EntityKind::Url { .. } => "Url",
        }
    }

    pub fn view(vec: Vec<Self>) {
        let map = group_entities(vec);

        for (key, items) in map {
            let title = key;
            match key {
                "Node" => {
                    let nodes = extract_inner(items, Entity::into_inner_node);
                    Node::view(nodes, title);
                }
                "Domain" => {
                    let domains = extract_inner(items, Entity::into_inner_domain);
                    Domain::view(domains, title);
                }
                "Url" => {
                    let urls = extract_inner(items, Entity::into_inner_url);
                    Url::view(urls, title);
                }
                _ => todo!(),
            }
        }
    }

    fn into_inner_url(self) -> Option<Url> {
        if let EntityKind::Url { inner, .. } = self.kind {
            Some(inner)
        } else {
            None
        }
    }

    fn into_inner_node(self) -> Option<Node> {
        if let EntityKind::Node { inner, .. } = self.kind {
            Some(inner)
        } else {
            None
        }
    }

    fn into_inner_domain(self) -> Option<Domain> {
        if let EntityKind::Domain { inner, .. } = self.kind {
            Some(inner)
        } else {
            None
        }
    }
}

fn extract_inner<T, F>(entities: Vec<Entity>, f: F) -> Vec<T>
where
    F: Fn(Entity) -> Option<T>,
{
    entities.into_iter().filter_map(f).collect()
}

fn group_entities(entities: Vec<Entity>) -> HashMap<&'static str, Vec<Entity>> {
    let mut map: HashMap<&'static str, Vec<Entity>> = HashMap::new();
    for e in entities {
        map.entry(e.type_name()).or_default().push(e);
    }
    map
}

pub trait EntityItem {
    #[allow(dead_code)]
    fn hash<S: Into<String>>(self, text: S) -> Self;
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
