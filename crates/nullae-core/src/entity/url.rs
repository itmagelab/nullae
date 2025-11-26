use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::{SHORT_HASH, entity::EntityItem, prelude::*};

#[derive(Serialize, Deserialize, Debug, Tabled, Indexable, Entity)]
pub struct Url {
    pub(crate) hash: HashID,
    #[index]
    pub(crate) slug: String,
    pub(crate) url: url::Url,
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = self.hash.short_hash();
        writeln!(f, "Short Url ➤ {}", desc)?;
        writeln!(f, "  → Url: {}", self.url)?;
        writeln!(f, "  → Hash: {}", self.hash)
    }
}

impl EntityItem for Url {}

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
    use crate::prelude::*;

    #[tokio::test]
    async fn it_works() {
        dotenvy::dotenv().ok();

        let ctx = Context::new().unwrap();
        let url = Url::create("https://ya.ru/some?param=1", &ctx)
            .await
            .unwrap();
        url.delete(&ctx).await.unwrap();
    }
}
