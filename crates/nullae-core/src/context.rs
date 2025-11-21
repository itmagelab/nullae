use crate::repository::Repository;

/// Application context that holds shared resources and configuration.
/// This should be passed through all application layers according to conventions.md.
#[derive(Debug)]
pub struct Context {
    repository: Repository,
}

impl Context {
    /// Creates a new Context with initialized Repository.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Required environment variables are not set (NULLAE_CONSUL_URL)
    /// - Repository initialization fails
    pub fn new() -> anyhow::Result<Self> {
        let repository = Repository::new()?;
        Ok(Self { repository })
    }

    /// Returns a reference to the Repository.
    pub fn repository(&self) -> &Repository {
        &self.repository
    }
}
