//! Handles the initial configuration for the DID Cache Client.
//!
//! Call the [ClientConfigBuilder] to create a new configuration.
//!
//! Example: Running in local mode with defaults:
//! ```rust
//! use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
//! let config = ClientConfigBuilder::default().build();
//! ```
//!
//! Example: Running in network mode with custom settings:
//! ```rust
//! use affinidi_did_resolver_cache_sdk::config::ClientConfigBuilder;
//! let config = ClientConfigBuilder::default()
//!     .with_network_mode("ws://127.0.0.1:8080/did/v1/ws")
//!     .with_cache_capacity(500)
//!     .with_cache_ttl(60)
//!     .with_network_timeout(10000)
//!     .with_network_cache_limit_count(200)
//!     .build();
//! ```
//!

use std::time::Duration;

/// Private Configuration for the client.
///
/// Use the [ClientConfigBuilder] to create a new configuration.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub(crate) service_address: Option<String>,
    pub(crate) cache_capacity: u32,
    pub(crate) cache_ttl: u32,
    pub(crate) network_timeout: Duration,
    pub(crate) network_cache_limit_count: u32,
    pub(crate) max_did_parts: usize,
    pub(crate) max_did_size_in_kb: f64,
}

/// Config Builder to construct options required for the client.
/// You must at least set the service address.
///
/// - service_address: REQUIRED: The address of the service to connect to.
/// - cache_capacity: The maximum number of items to store in the local cache (default: 100).
/// - cache_ttl: The time-to-live in seconds for each item in the local cache (default: 300 (5 Minutes)).
/// - network_timeout: The timeout for network requests in milliseconds (default: 5000 (5 seconds)).
/// - network_cache_limit_count: The maximum number of items to store in the network cache (default: 100).
pub struct ClientConfigBuilder {
    service_address: Option<String>,
    cache_capacity: u32,
    cache_ttl: u32,
    network_timeout: u32,
    network_cache_limit_count: u32,
    max_did_parts: usize,
    max_did_size_in_kb: f64,
}

impl Default for ClientConfigBuilder {
    fn default() -> Self {
        Self {
            service_address: None,
            cache_capacity: 100,
            cache_ttl: 300,
            network_timeout: 5000,
            network_cache_limit_count: 100,
            max_did_parts: 5,
            max_did_size_in_kb: 1.0,
        }
    }
}

impl ClientConfigBuilder {
    /// Enables network mode and sets the service address.
    /// Example: `ws://127.0.0.1:8080/did/v1/ws`
    pub fn with_network_mode(mut self, service_address: &str) -> Self {
        self.service_address = Some(service_address.into());
        self
    }

    /// Set the cache capacity (approx)
    /// Default: 100 items
    pub fn with_cache_capacity(mut self, cache_capacity: u32) -> Self {
        self.cache_capacity = cache_capacity;
        self
    }

    /// Set the time-to-live in seconds for each item in the local cache.
    /// Default: 300 (5 Minutes)
    pub fn with_cache_ttl(mut self, cache_ttl: u32) -> Self {
        self.cache_ttl = cache_ttl;
        self
    }

    /// Set the timeout for network requests in milliseconds.
    /// Default: 5000 (5 seconds)
    pub fn with_network_timeout(mut self, network_timeout: u32) -> Self {
        self.network_timeout = network_timeout;
        self
    }

    /// Set the network cache limit count
    /// Default: 100 items
    pub fn with_network_cache_limit_count(mut self, limit_count: u32) -> Self {
        self.network_cache_limit_count = limit_count;
        self
    }

    /// Set maximum number of parts after splitting method-specific-id on "."
    /// Default: 5 parts
    pub fn with_max_did_parts(mut self, max_did_parts: usize) -> Self {
        self.max_did_parts = max_did_parts;
        self
    }

    /// Set maximum size in KB of did to be resolved as FLOAT
    /// Default: 5 parts
    pub fn with_max_did_size_in_kb(mut self, max_did_size_in_kb: f64) -> Self {
        self.max_did_size_in_kb = max_did_size_in_kb;
        self
    }

    /// Build the [ClientConfig].
    pub fn build(self) -> ClientConfig {
        ClientConfig {
            service_address: self.service_address,
            cache_capacity: self.cache_capacity,
            cache_ttl: self.cache_ttl,
            network_timeout: Duration::from_millis(self.network_timeout.into()),
            network_cache_limit_count: self.network_cache_limit_count,
            max_did_parts: self.max_did_parts,
            max_did_size_in_kb: self.max_did_size_in_kb,
        }
    }
}
