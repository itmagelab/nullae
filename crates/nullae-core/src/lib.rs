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
    fn wrap(&self, max_width: usize) -> String;
}

impl Hashable for str {
    fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.as_bytes());
        hex::encode(hasher.finalize())
    }
    fn wrap(&self, max_width: usize) -> String {
        if max_width == 0 {
            return self.to_string();
        }

        let mut result = Vec::new();
        let mut current_line = String::new();

        for word in self.split_whitespace() {
            let word_len = word.chars().count();

            if current_line.is_empty() {
                if word_len > max_width {
                    result.push(word.to_string());
                } else {
                    current_line = word.to_string();
                }
            } else {
                let current_len = current_line.chars().count();
                let new_len = current_len + 1 + word_len;

                if new_len <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    result.push(current_line);
                    if word_len > max_width {
                        result.push(word.to_string());
                        current_line = String::new();
                    } else {
                        current_line = word.to_string();
                    }
                }
            }
        }

        if !current_line.is_empty() {
            result.push(current_line);
        }

        result.join("\n")
    }
}

impl Hashable for String {
    fn hash(&self) -> String {
        self.as_str().hash()
    }
    fn wrap(&self, max_width: usize) -> String {
        self.as_str().wrap(max_width)
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

    #[test]
    fn test_wrap_text_basic() {
        let text = "This is a very long line that needs to be wrapped";
        let wrapped = str::wrap(text, 20);

        for line in wrapped.lines() {
            assert!(line.len() <= 20, "Line '{}' exceeds max width", line);
        }

        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn test_wrap_text_short() {
        let text = "Short text";
        let wrapped = str::wrap(text, 50);
        assert_eq!(wrapped, text);
    }

    #[test]
    fn test_wrap_text_exact_width() {
        let text = "Hello world";
        let wrapped = str::wrap(text, 11);
        assert_eq!(wrapped, "Hello world");
    }

    #[test]
    fn test_wrap_text_long_word() {
        let text = "This verylongwordthatexceedswidth fits";
        let wrapped = str::wrap(text, 10);

        assert!(wrapped.contains("verylongwordthatexceedswidth"));
    }

    #[test]
    fn test_wrap_text_empty() {
        let text = "";
        let wrapped = str::wrap(text, 10);
        assert_eq!(wrapped, "");
    }

    #[test]
    fn test_wrap_text_zero_width() {
        let text = "Some text";
        let wrapped = str::wrap(text, 0);
        assert_eq!(wrapped, text);
    }
}
