use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::{SHORT_HASH, entity::EntityItem, prelude::*};

const DOMAIN: &str = "https://0ae.ru";

#[derive(Serialize, Deserialize, Debug, Tabled, Indexable, Entity)]
pub struct Url {
    pub(crate) hash: String,
    #[index]
    pub(crate) slug: String,
    pub(crate) url: url::Url,
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = self.hash[..SHORT_HASH].to_string();
        writeln!(f, "Short Url ➤ {}", desc)?;
        writeln!(f, "  → Url: {}", self.url)?;
        writeln!(f, "  → Hash: {}", self.hash)
    }
}

impl EntityItem for Url {
    fn hash<S>(mut self, text: S) -> Self
    where
        S: Into<String>,
    {
        self.hash = text.into();
        self
    }
}

impl Url {
    fn new(url: &str) -> anyhow::Result<Self> {
        let hash = format!("{}|", url).hash();
        let slug = hash[..SHORT_HASH].to_string();
        let url = url::Url::parse(url)?;
        Ok(Self { hash, slug, url })
    }

    pub async fn create(name: &str, repository: &Repository) -> anyhow::Result<Self> {
        let url = Self::new(name)?;
        url.index()?.save(repository).await?;
        repository.create(&url.into()).await?.try_into()
    }

    pub async fn delete(self, repository: &Repository) -> anyhow::Result<()> {
        let entity: Entity = self.into();
        entity.delete(repository).await?;
        Ok(())
    }

    pub fn url(&self) -> String {
        self.url.to_string()
    }

    pub fn short_url(&self) -> String {
        let slug = &self.slug;
        format!("{DOMAIN}/{slug}")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[tokio::test]
    async fn it_works() {
        let repository = Repository::new().unwrap();
        let url = Url::create("https://ya.ru/some?param=1", &repository)
            .await
            .unwrap();
        url.delete(&repository).await.unwrap();
    }
}
