pub mod context;
pub(crate) mod entity;
pub mod handler;
pub(crate) mod index;
pub mod prelude;
pub mod repository;

use crate::index::Index;
use serde::{Deserialize, Serialize};

pub(crate) const BASE_PATH: &str = "0ae";
pub(crate) const SHORT_HASH: usize = 8;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Metadata {
    created_at: chrono::NaiveDateTime,
    updated_at: Option<chrono::NaiveDateTime>,
    children: Option<Vec<String>>,
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

pub trait Hashable {
    fn hash(&self) -> String;
}

impl Hashable for str {
    fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.as_bytes());
        hex::encode(hasher.finalize())
    }
}

impl Hashable for String {
    fn hash(&self) -> String {
        self.as_str().hash()
    }
}

pub trait Indexable {
    fn index(&self) -> anyhow::Result<Index>;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn random_nodes_domains() {
        dotenvy::dotenv().ok();

        use futures::stream::{self, StreamExt};

        let ctx = Context::new().unwrap();

        // Create 5 different domains
        let domains: Vec<Domain> = stream::iter(0..5)
            .map(|i| {
                let ctx = &ctx;
                async move { Domain::create(&format!("domain-{}", i), ctx).await.unwrap() }
            })
            .buffer_unordered(5)
            .collect()
            .await;

        // Create 100 nodes randomly distributed across domains
        let nodes: Vec<Node> = stream::iter(0..100)
            .map(|i| {
                let ctx = &ctx;
                let domain = domains[i % 5].clone(); // Round-robin distribution
                async move {
                    Node::create(&format!("node-{}", i), &domain, ctx)
                        .await
                        .unwrap()
                }
            })
            .buffer_unordered(50)
            .collect()
            .await;

        // Cleanup: delete all nodes
        stream::iter(nodes)
            .map(|node| {
                let ctx = &ctx;
                async move { node.delete(ctx).await.unwrap() }
            })
            .buffer_unordered(50)
            .collect::<()>()
            .await;

        // Cleanup: delete all domains
        stream::iter(domains)
            .map(|domain| {
                let ctx = &ctx;
                async move { domain.delete(ctx).await.unwrap() }
            })
            .buffer_unordered(5)
            .collect::<()>()
            .await;
    }
}
