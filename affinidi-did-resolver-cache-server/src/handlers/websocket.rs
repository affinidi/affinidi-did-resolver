use affinidi_did_resolver_cache_sdk::networking::{
    WSRequest, WSResponse, WSResponseError, WSResponseType,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use tokio::select;
use tracing::{debug, info, span, warn, Instrument};

use crate::SharedData;

// Handles the switching of the protocol to a websocket connection
pub async fn websocket_handler(
    //session: Session,
    ws: WebSocketUpgrade,
    State(state): State<SharedData>,
) -> impl IntoResponse {
    let _span = span!(
        tracing::Level::DEBUG,
        "websocket_handler",
        // session = session.session_id
    );
    /*async move { ws.on_upgrade(move |socket| handle_socket(socket, state, session)) }
    .instrument(_span)
    .await*/

    async move { ws.on_upgrade(move |socket| handle_socket(socket, state)) }
        .instrument(_span)
        .await
}

/// WebSocket state machine. This is spawned per connection.
//async fn handle_socket(mut socket: WebSocket, state: SharedData, session: Session) {
async fn handle_socket(mut socket: WebSocket, state: SharedData) {
    let _span = span!(
        tracing::Level::DEBUG,
        "handle_socket",
        //session = session.session_id
    );
    async move {
        state.stats().await.increment_ws_opened();
        info!("Websocket connection established");

        loop {
            select! {
                value = socket.recv() => {
                    if let Some(msg) = value {
                        if let Ok(msg) = msg {
                            if let Message::Text(msg) = msg {
                                debug!("ws: Received text message: {:?}", msg);
                                let request: WSRequest = match serde_json::from_str(&msg) {
                                    Ok(request) => request,
                                    Err(e) => {
                                        warn!("ws: Error parsing message: {:?}", e);
                                        break;
                                    }
                                };

                                match state.resolver.resolve(&request.did).await {
                                    Ok(response) => {
                                        let message = WSResponseType::Response(WSResponse {
                                            did: response.did.clone(),
                                            hash: response.did_hash,
                                            document: response.doc,
                                        });
                                        let mut stats = state.stats().await;
                                        stats.increment_resolver_success();
                                        if response.cache_hit { stats.increment_cache_hit();}
                                        stats.increment_did_method_success(response.method);
                                        drop(stats);
                                        info!("resolved DID: ({}) cache_hit?({})", response.did, response.cache_hit);
                                        if let Err(e) = socket.send(Message::Text(serde_json::to_string(&message).unwrap())).await {
                                            warn!("ws: Error sending response: {:?}", e);
                                            break;
                                        } else {
                                            debug!("Sent response: {:?}", message);
                                        }
                                    }
                                    Err(e) => {
                                        // Couldn't resolve the DID, send an error back
                                        warn!("Couldn't resolve DID: ({}) Reason: {}", &request.did, e);
                                        state.stats().await.increment_resolver_error();
                                        if let Err(e) = socket.send(Message::Text(serde_json::to_string(&WSResponseType::Error(WSResponseError {did: request.did, hash: request.hash, error: e.to_string()})).unwrap())).await {
                                            warn!("ws: Error sending error response: {:?}", e);
                                            break;
                                        }
                                    }
                                }
                            } else {
                                warn!("Received non-text message, ignoring");
                                continue;
                            }
                        }
                    } else {
                        debug!("Received None, closing connection");
                        break;
                    }
                }
            }
        }

        // We're done, close the connection
        state.stats().await.increment_ws_closed();

        info!("Websocket connection closed");
    }
    .instrument(_span)
    .await
}
