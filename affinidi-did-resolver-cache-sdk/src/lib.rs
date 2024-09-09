//! DID Universal Resolver Cache Client SDK
//!
//! Used to easily connect to the DID Universal Resolver Cache.
//!
use blake2::{Blake2s256, Digest};
use config::ClientConfig;
use errors::DIDCacheError;
use moka::future::Cache;
use networking::{
    network::{NetworkTask, WSCommands},
    WSRequest,
};
use ssi::dids::Document;
use std::{fmt, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tracing::debug;

pub mod config;
pub mod document;
pub mod errors;
pub mod networking;
mod resolver;

/// DID Methods supported by the DID Universal Resolver Cache
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DIDMethod {
    ETHR,
    JWK,
    KEY,
    PEER,
    PKH,
    WEB,
}

/// Helper function to convert a DIDMethod to a string
impl fmt::Display for DIDMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DIDMethod::ETHR => write!(f, "ethr"),
            DIDMethod::JWK => write!(f, "jwk"),
            DIDMethod::KEY => write!(f, "key"),
            DIDMethod::PEER => write!(f, "peer"),
            DIDMethod::PKH => write!(f, "pkh"),
            DIDMethod::WEB => write!(f, "web"),
        }
    }
}

/// Helper function to convert a string to a DIDMethod
impl TryFrom<String> for DIDMethod {
    type Error = DIDCacheError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl TryFrom<&str> for DIDMethod {
    type Error = DIDCacheError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "ethr" => Ok(DIDMethod::ETHR),
            "jwk" => Ok(DIDMethod::JWK),
            "key" => Ok(DIDMethod::KEY),
            "peer" => Ok(DIDMethod::PEER),
            "pkh" => Ok(DIDMethod::PKH),
            "web" => Ok(DIDMethod::WEB),
            _ => Err(DIDCacheError::UnsupportedMethod(value.to_string())),
        }
    }
}

pub struct ResolveResponse {
    pub did: String,
    pub method: DIDMethod,
    pub did_hash: String,
    pub doc: Document,
    pub cache_hit: bool,
}

// ***************************************************************************

/// [DIDCacheClient] is how you interact with the DID Universal Resolver Cache
/// config: Configuration for the SDK
/// cache: Local cache for resolved DIDs
/// network_task: OPTIONAL: Task to handle network requests
/// network_rx: OPTIONAL: Channel to listen for responses from the network task
#[derive(Clone)]
pub struct DIDCacheClient {
    config: ClientConfig,
    cache: Cache<String, Document>,
    network_task_tx: Option<mpsc::Sender<WSCommands>>,
    network_task_rx: Option<Arc<Mutex<mpsc::Receiver<WSCommands>>>>,
}

impl DIDCacheClient {
    /// Create a new DIDCacheClient with configuration generated from [ClientConfigBuilder](config::ClientConfigBuilder)
    ///
    /// Will return an error if the configuration is invalid.
    ///
    /// Establishes websocket connection and sets up the cache.
    pub async fn new(config: ClientConfig) -> Result<Self, DIDCacheError> {
        // Create the initial cache
        let cache = Cache::builder()
            .max_capacity(config.cache_capacity.into())
            .time_to_live(Duration::from_secs(config.cache_ttl.into()))
            .build();

        let mut client = Self {
            config,
            cache,
            network_task_tx: None,
            network_task_rx: None,
        };

        if client.config.service_address.is_some() {
            // Running in network mode

            // Channel to communicate from SDK to network task
            let (sdk_tx, mut task_rx) = mpsc::channel(32);
            // Channel to communicate from network task to SDK
            let (task_tx, sdk_rx) = mpsc::channel(32);

            client.network_task_tx = Some(sdk_tx);
            client.network_task_rx = Some(Arc::new(Mutex::new(sdk_rx)));
            // Start the network task
            let _config = client.config.clone();
            tokio::spawn(async move {
                let _ = NetworkTask::run(_config, &mut task_rx, &task_tx).await;
            });

            if let Some(arc_rx) = client.network_task_rx.as_ref() {
                // Wait for the network task to be ready
                let mut rx = arc_rx.lock().await;
                rx.recv().await.unwrap();
            }
        }

        Ok(client)
    }

    /// Front end for resolving a DID
    /// Will check the cache first, and if not found, will resolve the DID
    /// Returns the initial DID, the hashed DID, and the resolved DID Document
    /// NOTE: The DID Document id may be different to the requested DID due to the DID having been updated.
    ///       The original DID should be in the `also_known_as` field of the DID Document.
    pub async fn resolve(&self, did: &str) -> Result<ResolveResponse, DIDCacheError> {
        //let did_hash = sha256::digest(did);
        let mut hasher = Blake2s256::new();
        hasher.update(did);
        let did_hash = format!("{:x}", hasher.finalize());

        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() < 3 {
            return Err(DIDCacheError::DIDError(format!(
                "did isn't to spec! did ({})",
                did
            )));
        }

        // Check if the DID is in the cache
        if let Some(doc) = self.cache.get(&did_hash).await {
            debug!("found did ({}) in cache", did);
            Ok(ResolveResponse {
                did: did.to_string(),
                method: parts[1].try_into()?,
                did_hash,
                doc,
                cache_hit: true,
            })
        } else {
            debug!("did ({}) NOT in cache hash ({})", did, did_hash);
            // If the DID is not in the cache, resolve it (local or via network)
            let doc = if self.config.service_address.is_none() {
                debug!("resolving did ({}) locally", did);
                self.local_resolve(did, &parts).await?
            } else {
                self.network_resolve(did, &did_hash).await?
            };

            debug!("adding did ({}) to cache ({})", did, did_hash);
            self.cache.insert(did_hash.clone(), doc.clone()).await;
            Ok(ResolveResponse {
                did: did.to_string(),
                method: parts[1].try_into()?,
                did_hash,
                doc,
                cache_hit: false,
            })
        }
    }

    /// If you want to interact directly with the DID Document cache
    /// This will return a clone of the cache (the clone is cheap, and the cache is shared)
    /// For example, accessing cache statistics or manually inserting a DID Document
    pub fn get_cache(&self) -> Cache<String, Document> {
        self.cache.clone()
    }

    /// Stops the network task if it is running and removes any resources
    pub fn stop(&self) {
        if let Some(tx) = self.network_task_tx.as_ref() {
            let _ = tx.blocking_send(WSCommands::Exit);
        }
    }

    /// Removes the specified DID from the cache
    /// Returns the removed DID Document if it was in the cache, or None if it was not
    pub async fn remove(&self, did: &str) -> Option<Document> {
        //let did_hash = sha256::digest(did);
        let mut hasher = Blake2s256::new();
        hasher.update(did);
        let did_hash = format!("{:x}", hasher.finalize());
        self.cache.remove(&did_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DID_KEY: &str = "did:key:z6MkiToqovww7vYtxm1xNM15u9JzqzUFZ1k7s7MazYJUyAxv";

    async fn basic_local_client() -> DIDCacheClient {
        let config = config::ClientConfigBuilder::default().build();
        DIDCacheClient::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn remove_existing_cached_did() {
        let client = basic_local_client().await;

        // Resolve a DID which automatically adds it to the cache
        let response = client.resolve(DID_KEY).await.unwrap();
        let removed_doc = client.remove(DID_KEY).await;
        assert_eq!(removed_doc, Some(response.doc));
    }

    #[tokio::test]
    async fn remove_non_existing_cached_did() {
        let client = basic_local_client().await;

        // We haven't resolved the cache, so it shouldn't be in the cache
        let removed_doc = client.remove(DID_KEY).await;
        assert_eq!(removed_doc, None);
    }
}
