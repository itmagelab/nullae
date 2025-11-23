pub mod domain;
pub mod ip;
pub mod node;
pub mod url;

use crate::prelude::*;

use crate::{BASE_PATH, SHORT_HASH};
use serde::{Deserialize, Serialize};
use tabled::{
    Tabled,
    settings::{
        Format, Modify, Style, Width,
        object::{Columns, Object, Rows, Segment},
        style::{HorizontalLine, LineText, VerticalLine},
    },
};

use crate::Metadata;

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
    pub(crate) fn add_child(&mut self, hash: &str) {
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

    pub fn has_children(&self) -> bool {
        self.metadata.children.is_some()
    }

    pub async fn delete(self, ctx: &Context) -> anyhow::Result<()> {
        if self.has_children() {
            anyhow::bail!("Can't delete entity with children");
        }

        // Get index before deleting entity
        let index = Index::from_entity(&self)?;

        match self.kind {
            EntityKind::Node { inner } => inner.delete(ctx).await?,
            _ => {
                // Delete entity from repository
                let url = ctx.storage().build_url(&self.path());
                ctx.storage().pool().delete(&url).send().await?;

                // Purge index entries
                index.purge(ctx).await?;
            }
        }
        Ok(())
    }

    pub(crate) fn payload(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    pub(crate) fn hash(&self) -> &str {
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
