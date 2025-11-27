pub mod context;
pub(crate) mod entity;
pub mod handler;
pub(crate) mod index;
pub mod prelude;
pub mod storage;

use crate::index::Index;

pub(crate) const BASE_PATH: &str = "0ae";
pub(crate) const SHORT_HASH: usize = 8;

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

    #[tokio::test]
    async fn test_some() {
        dotenvy::dotenv().ok();
        let ctx = Context::new().unwrap();
        let domain = Domain::create("testing", &ctx).await.unwrap();
        let mut node = Node::create("local-1", &domain, &ctx).await.unwrap();
        node.collect_host_info(&ctx).await.unwrap();
        let node = node.save(&ctx).await.unwrap();
        node.delete(&ctx).await.unwrap();
        domain.delete(&ctx).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn random_nodes_domains() {
        dotenvy::dotenv().ok();

        use futures::stream::{self, StreamExt};

        let ctx = Context::new().unwrap();

        let domains: Vec<Domain> = stream::iter(0..10)
            .map(|i| {
                let ctx = &ctx;
                async move { Domain::create(&format!("domain-{}", i), ctx).await.unwrap() }
            })
            .buffer_unordered(5)
            .collect()
            .await;

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

        Node::delete_batch(nodes, &ctx, 10).await.unwrap();

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
