/*!
DID Universal Resolver Cache Client SDK

Used to easily connect to the DID Universal Resolver Cache.

# Crate features
As this crate can be used either natively or in a WASM environment, the following features are available:
* **local**
    **default** - Enables the local mode of the SDK. This is the default mode.
* **network**
    * Enables the network mode of the SDK. This mode requires a run-time service address to connect to.
    * This feature is NOT supported in a WASM environment. Will cause a compile error if used in WASM.
*/

#[cfg(all(feature = "network", target_arch = "wasm32"))]
compile_error!("Cannot enable both features at the same time");

use blake2::{Blake2s256, Digest};
use config::ClientConfig;
use errors::DIDCacheError;
use moka::future::Cache;
#[cfg(feature = "network")]
use networking::{
    network::{NetworkTask, WSCommands},
    WSRequest,
};
use ssi::dids::Document;
#[cfg(feature = "network")]
use std::sync::Arc;
use std::{fmt, time::Duration};
#[cfg(feature = "network")]
use tokio::sync::{mpsc, Mutex};
use tracing::debug;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

pub mod config;
pub mod document;
pub mod errors;
#[cfg(feature = "network")]
pub mod networking;
mod resolver;

const BYTES_PER_KILO_BYTE: f64 = 1000.0;

/// DID Methods supported by the DID Universal Resolver Cache
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[wasm_bindgen]
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
#[wasm_bindgen(getter_with_clone)]
#[derive(Clone)]
pub struct DIDCacheClient {
    config: ClientConfig,
    cache: Cache<String, Document>,
    #[cfg(feature = "network")]
    network_task_tx: Option<mpsc::Sender<WSCommands>>,
    #[cfg(feature = "network")]
    network_task_rx: Option<Arc<Mutex<mpsc::Receiver<WSCommands>>>>,
}

impl DIDCacheClient {
    /// Front end for resolving a DID
    /// Will check the cache first, and if not found, will resolve the DID
    /// Returns the initial DID, the hashed DID, and the resolved DID Document
    /// NOTE: The DID Document id may be different to the requested DID due to the DID having been updated.
    ///       The original DID should be in the `also_known_as` field of the DID Document.
    pub async fn resolve(&self, did: &str) -> Result<ResolveResponse, DIDCacheError> {
        let did_size_in_kb = did.len() as f64 / BYTES_PER_KILO_BYTE;

        // If DID's size is greater than 1KB we don't resolve it
        if did_size_in_kb > self.config.max_did_size_in_kb {
            return Err(DIDCacheError::DIDError(format!(
                "The DID size of {:.3}KB exceeds the limit of {1}KB. Please ensure the size is less than {1}KB.",
                did_size_in_kb, self.config.max_did_size_in_kb
            )));
        }

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

        let key_parts: Vec<&str> = parts.last().unwrap().split(".").collect();
        if key_parts.len() > self.config.max_did_parts {
            return Err(DIDCacheError::DIDError(format!(
                "The total number of keys and/or services must be less than or equal to {:?}, but {:?} were found.",
                self.config.max_did_parts,
                parts.len()
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
            #[cfg(feature = "network")]
            let doc = {
                if self.config.service_address.is_some() {
                    self.network_resolve(did, &did_hash).await?
                } else {
                    self.local_resolve(did, &parts).await?
                }
            };

            #[cfg(not(feature = "network"))]
            let doc = self.local_resolve(did, &parts).await?;

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
    #[cfg(feature = "network")]
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

/// Following are the WASM bindings for the DIDCacheClient
#[wasm_bindgen]
impl DIDCacheClient {
    /// Create a new DIDCacheClient with configuration generated from [ClientConfigBuilder](config::ClientConfigBuilder)
    ///
    /// Will return an error if the configuration is invalid.
    ///
    /// Establishes websocket connection and sets up the cache.
    // using Self instead of DIDCacheClient leads to E0401 errors in dependent crates
    // this is due to wasm_bindgen generated code (check via `cargo expand`)
    pub async fn new(config: ClientConfig) -> Result<DIDCacheClient, DIDCacheError> {
        // Create the initial cache
        let cache = Cache::builder()
            .max_capacity(config.cache_capacity.into())
            .time_to_live(Duration::from_secs(config.cache_ttl.into()))
            .build();

        #[cfg(feature = "network")]
        let mut client = Self {
            config,
            cache,
            network_task_tx: None,
            network_task_rx: None,
        };
        #[cfg(not(feature = "network"))]
        let client = Self { config, cache };

        #[cfg(feature = "network")]
        {
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
        }

        Ok(client)
    }

    pub async fn wasm_resolve(&self, did: &str) -> Result<JsValue, DIDCacheError> {
        let response = self.resolve(did).await?;

        match serde_wasm_bindgen::to_value(&response.doc) {
            Ok(values) => Ok(values),
            Err(err) => Err(DIDCacheError::DIDError(format!(
                "Error serializing DID Document: {}",
                err
            ))),
        }
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
