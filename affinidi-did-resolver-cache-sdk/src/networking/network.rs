//! NetworkTask handles the communication with the network.
//! This runs as a separate task in the background.
//! Allows for multiplexing of multiple requests/responses to the network.
//!
//! The SDK communicates via a MPSC channel to this task.
//! The remote server communicates via a websocket connection.
//!

use std::time::Duration;

use crate::{config::ClientConfig, errors::DIDCacheError, WSRequest};
use blake2::{Blake2s256, Digest};
use futures_util::{SinkExt, StreamExt};
use ssi::dids::Document;
use tokio::{
    net::TcpStream,
    select,
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    time::sleep,
};
#[cfg(feature = "network")]
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, span, warn, Instrument, Level};

use super::{request_queue::RequestList, WSResponseType};

/// WSCommands are the commands that can be sent between the SDK and the network task
/// Connected: Signals that the websocket is connected
/// Exit: Exits the websocket handler
/// Send: Sends the response string to the websocket (Channel, ID, WSRequest)
/// ResponseReceived: Response received from the websocket
/// ErrorReceived: Error received from the remote server
/// NotFound: Response not found in the cache
/// TimeOut: SDK request timed out, contains ID and did_hash we were looking for
#[derive(Debug)]
pub(crate) enum WSCommands {
    Connected,
    Exit,
    Send(Responder, String, WSRequest),
    ResponseReceived(Box<Document>),
    ErrorReceived(String),
    TimeOut(String, String),
}

pub(crate) type Responder = oneshot::Sender<WSCommands>;

/// NetworkTask handles the communication with the network.
/// This runs as a separate task in the background.
///
/// sdk_tx_channel: Sender<WSCommands> - Channel to send commands to the network task from the SDK
/// sdk_rx_channel: Rc<Receiver<WSCommands>> - Channel to receive commands from the network task
/// task_rx_channel: Rc<Receiver<WSCommands>> - PRIVATE. Channel to receive commands from the SDK
/// task_tx_channel: Sender<WSCommands> - PRIVATE. Channel to send commands to the SDK
/// websocket: Option<Rc<WebSocketStream<MaybeTlsStream<TcpStream>>>> - PRIVATE. The websocket connection itself
pub(crate) struct NetworkTask {
    config: ClientConfig,
    service_address: String,
    cache: RequestList,
    sdk_tx: Sender<WSCommands>,
}

impl NetworkTask {
    pub async fn run(
        config: ClientConfig,
        sdk_rx: &mut Receiver<WSCommands>,
        sdk_tx: &Sender<WSCommands>,
    ) -> Result<(), DIDCacheError> {
        let _span = span!(Level::INFO, "network_task");
        async move {
            debug!("Starting...");

            let service_address = if let Some(service_address) = &config.service_address {
                service_address.to_string()
            } else {
                return Err(DIDCacheError::ConfigError(
                    "Running in local mode, yet network service called!".to_string(),
                ));
            };

            let cache = RequestList::new(&config);

            let mut network_task = NetworkTask {
                config,
                service_address,
                cache,
                sdk_tx: sdk_tx.clone(),
            };

            let mut websocket = network_task.ws_connect().await?;

            loop {
                select! {
                    value = websocket.next() => {
                        if network_task.ws_recv(value).is_err() {
                            // Reset the connection
                            websocket = network_task.ws_connect().await?;
                        }
                    },
                    value = sdk_rx.recv(), if !network_task.cache.is_full() => {
                        if let Some(cmd) = value {
                            match cmd {
                                WSCommands::Send(channel, uid, request) => {
                                    let mut hasher = Blake2s256::new();
                                    hasher.update(request.did.clone());
                                    let did_hash = format!("{:x}", hasher.finalize());
                                    if network_task.cache.insert(did_hash, &uid, channel) {
                                        let _ = network_task.ws_send(&mut websocket, &request).await;
                                    }
                                }
                                WSCommands::TimeOut(uid, did_hash) => {
                                    let _ = network_task.cache.remove(&did_hash, Some(uid));
                                }
                                WSCommands::Exit => {
                                    debug!("Exiting...");
                                    return Ok(());
                                }
                                _ => {
                                    debug!("Invalid command received: {:?}", cmd);
                                }
                            }
                        }
                    }
                }
            }
        }
        .instrument(_span)
        .await
    }

    /// Creates the connection to the remote server via a websocket
    /// If timeouts or errors occur, it will backoff and retry
    /// NOTE: Increases in 5 second increments up to 60 seconds
    async fn ws_connect(
        &self,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, DIDCacheError> {
        async fn _handle_backoff(backoff: Duration) -> Duration {
            let b = if backoff.as_secs() < 60 {
                backoff.saturating_add(Duration::from_secs(5))
            } else {
                backoff
            };

            debug!("connect backoff: {} Seconds", b.as_secs());
            sleep(b).await;
            b
        }

        let _span = span!(Level::DEBUG, "ws_connect", server = self.service_address);
        async move {
            // Connect to the DID cache server
            let mut backoff = Duration::from_secs(1);
            loop {
                debug!("Starting websocket connection");

                let connection = connect_async(&self.service_address);
                let timeout = tokio::time::sleep(self.config.network_timeout);

                select! {
                    conn = connection => {
                        match conn {
                            Ok((conn, _)) => {
                                debug!("Websocket connected");
                                self.sdk_tx.send(WSCommands::Connected).await.unwrap();
                                return Ok(conn)
                            }
                            Err(e) => {
                                error!("Error connecting to websocket: {:?}", e);
                                backoff = _handle_backoff(backoff).await;
                            }
                        }
                    }
                    _ = timeout => {
                        // Start backing off and retry
                        warn!("Connect timeout reached");
                        backoff = _handle_backoff(backoff).await;
                    }
                }
            }
        }
        .instrument(_span)
        .await
    }

    /// Sends the request to the remote server via the websocket
    async fn ws_send(
        &self,
        websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        request: &WSRequest,
    ) -> Result<(), DIDCacheError> {
        match websocket
            .send(serde_json::to_string(request).unwrap().into())
            .await
        {
            Ok(_) => {
                debug!("Request sent: {:?}", request);
                Ok(())
            }
            Err(e) => Err(DIDCacheError::TransportError(format!(
                "Couldn't send request to network_task. Reason: {}",
                e
            ))),
        }
    }

    /// Processes inbound websocket messages from the remote server
    fn ws_recv(
        &mut self,
        message: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    ) -> Result<(), DIDCacheError> {
        if let Some(response) = message {
            match response {
                Ok(msg) => {
                    if let Message::Text(msg) = msg {
                        let response: Result<WSResponseType, _> = serde_json::from_str(&msg);
                        match response {
                            Ok(WSResponseType::Response(response)) => {
                                debug!("Received response: {:?}", response.hash);
                                if let Some(channels) = self.cache.remove(&response.hash, None) {
                                    // Loop through and notify each registered channel
                                    for channel in channels {
                                        let _ = channel.send(WSCommands::ResponseReceived(
                                            Box::new(response.document.clone()),
                                        ));
                                    }
                                } else {
                                    warn!("Response not found in request list: {}", response.hash);
                                }
                            }
                            Ok(WSResponseType::Error(response)) => {
                                warn!(
                                    "Received error: did hash({}) Error: {:?}",
                                    response.hash, response.error
                                );
                                if let Some(channels) = self.cache.remove(&response.hash, None) {
                                    for channel in channels {
                                        let _ = channel.send(WSCommands::ErrorReceived(
                                            response.error.clone(),
                                        ));
                                    }
                                } else {
                                    warn!("Response not found in request list: {}", response.hash);
                                }
                            }
                            Err(e) => {
                                warn!("Error parsing message: {:?}", e);
                            }
                        }
                    } else {
                        warn!("Received non-text message, ignoring: {}", msg);
                    }
                }
                Err(e) => {
                    // Can't receive messages, reset the connection
                    error!("Error receiving message: {:?}", e);
                    return Err(DIDCacheError::TransportError(format!(
                        "Error receiving message: {:?}",
                        e
                    )));
                }
            }
        } else {
            warn!("No message received");
        }

        Ok(())
    }
}
