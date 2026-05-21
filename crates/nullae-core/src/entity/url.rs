use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::{SHORT_HASH, prelude::*};

#[derive(Serialize, Deserialize, Debug, Tabled, Indexable, Entity)]
pub struct Url {
    #[tabled(rename = "URL Hash")]
    pub(crate) hash: HashID,
    #[index]
    #[tabled(rename = "Short Slug")]
    pub(crate) slug: String,
    #[tabled(rename = "Target URL")]
    pub(crate) url: url::Url,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct UrlView {
    pub hash: String,
    pub slug: String,
    pub url: String,
}

impl From<&Url> for UrlView {
    fn from(u: &Url) -> Self {
        UrlView {
            hash: u.hash.to_string(),
            slug: u.slug.clone(),
            url: u.url.to_string(),
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            crate::entity::render_tabled_card(self, "🔗  URL DETAILS")
        )
    }
}

impl Url {
    fn new(url: &str) -> anyhow::Result<Self> {
        if url.trim().is_empty() {
            anyhow::bail!("URL cannot be empty or whitespace-only");
        }

        let parsed_url = url::Url::parse(url)
            .map_err(|e| anyhow::anyhow!("Invalid URL format '{}': {}", url, e))?;

        let hash = HashID::from_str(&format!("{}|", url).hash())?;
        let slug = hash.as_hex()[..SHORT_HASH].to_string();

        Ok(Self {
            hash,
            slug,
            url: parsed_url,
        })
    }

    pub async fn create<S: Storage>(name: &str, ctx: &Context<S>) -> anyhow::Result<Self> {
        let url = Self::new(name)?;
        url.index()?.save(ctx).await?;
        ctx.storage().create(&url.into()).await?.try_into()
    }

    pub async fn delete<S: Storage>(self, ctx: &Context<S>) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(ctx).await?;
        Ok(())
    }

    pub fn url(&self) -> String {
        self.url.to_string()
    }

    pub fn short_url(&self) -> anyhow::Result<String> {
        let domain = std::env::var("NULLAE_DOMAIN")
            .map_err(|_| anyhow::anyhow!("NULLAE_DOMAIN environment variable is required"))?;
        let slug = &self.slug;
        Ok(format!("{domain}/{slug}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_lifecycle_in_storage() {
        let storage = InMemoryStorage::new();
        let ctx = Context::with_storage(storage);

        // 1. Create URL entity in storage
        let target = "https://example.com/search?q=rust";
        let url = Url::create(target, &ctx).await.unwrap();
        assert_eq!(url.url.as_str(), "https://example.com/search?q=rust");

        // 2. Fetch the URL from storage using slug
        let found = ctx.storage().find(&url.slug).await.unwrap();
        assert!(!found.is_empty());
        assert_eq!(found[0].hash(), &url.hash);

        let url_hash = url.hash.clone();

        // 3. Delete URL entity
        url.delete(&ctx).await.unwrap();

        // 4. Verify it's gone
        let fetched_after_delete = ctx.storage().get(&url_hash.as_hex()).await.unwrap();
        assert!(fetched_after_delete.is_none());
    }
}
