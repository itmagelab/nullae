use crate::storage::Storage;
use crate::storage::consul::Consul;

/// Application context that holds shared resources and configuration.
/// This should be passed through all application layers according to conventions.md.
pub struct Context {
    storage: Box<dyn Storage + Send + Sync>,
}

impl Context {
    /// Creates a new Context with initialized Consul storage.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required environment variables are not set (NULLAE_CONSUL_URL)
    /// - Storage initialization fails
    pub fn new() -> anyhow::Result<Self> {
        let storage = Consul::new()?;
        Ok(Self {
            storage: Box::new(storage),
        })
    }

    /// Returns a reference to the Storage.
    pub fn storage(&self) -> &dyn Storage {
        self.storage.as_ref()
    }
}
