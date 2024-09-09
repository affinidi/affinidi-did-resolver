use did_peer::DIDPeer;
use ssi::dids::{DIDEthr, DIDKey, DIDResolver, DIDWeb, Document, DID, DIDJWK, DIDPKH};
use tracing::error;

use crate::{errors::DIDCacheError, DIDCacheClient};

impl DIDCacheClient {
    /// Resolves a DID to a DID Document
    pub(crate) async fn local_resolve(
        &self,
        did: &str,
        parts: &[&str],
    ) -> Result<Document, DIDCacheError> {
        // Match the DID method

        match parts[1] {
            "ethr" => {
                let method = DIDEthr;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            "jwk" => {
                let method = DIDJWK;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            "key" => {
                let method = DIDKey;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            "peer" => {
                let method = DIDPeer;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            "pkh" => {
                let method = DIDPKH;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            "web" => {
                let method = DIDWeb;

                match method.resolve(DID::new::<str>(did).unwrap()).await {
                    Ok(res) => Ok(res.document.into_document()),
                    Err(e) => {
                        error!("Error: {:?}", e);
                        Err(DIDCacheError::DIDError(e.to_string()))
                    }
                }
            }
            _ => Err(DIDCacheError::DIDError(format!(
                "DID Method ({}) not supported",
                parts[1]
            ))),
        }
    }
}
