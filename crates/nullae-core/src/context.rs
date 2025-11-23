use crate::storage::Storage;
use crate::storage::consul::Consul;

/// Application context that holds shared resources and configuration.
/// This should be passed through all application layers according to conventions.md.
pub struct Context<S: Storage> {
    storage: S,
}

impl<S: Storage> Context<S> {
    /// Creates a new Context with the provided storage.
    pub fn with_storage(storage: S) -> Self {
        Self { storage }
    }

    /// Returns a reference to the Storage.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl Context<Consul> {
    /// Creates a new Context with initialized Consul storage.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required environment variables are not set (NULLAE_CONSUL_URL)
    /// - Storage initialization fails
    pub fn new() -> anyhow::Result<Self> {
        let storage = Consul::new()?;
        Ok(Self { storage })
    }
}

/// Type alias for the default Context with Consul storage.
/// This provides backward compatibility for existing code.
pub type AppContext = Context<Consul>;
