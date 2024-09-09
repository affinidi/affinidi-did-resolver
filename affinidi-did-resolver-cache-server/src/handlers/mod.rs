use crate::SharedData;
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};

pub(crate) mod websocket;

pub fn application_routes(shared_data: &SharedData) -> Router {
    let app = Router::new()
        // Inbound message handling from ATM clients
        // Websocket endpoint for clients
        .route("/ws", get(websocket::websocket_handler));

    Router::new()
        .nest("/did/v1/", app)
        .with_state(shared_data.to_owned())
}

pub async fn health_checker_handler(State(state): State<SharedData>) -> impl IntoResponse {
    let message: String = format!(
        "Affinidi Trust Network - DID Cache, Version: {}, Started: UTC {}",
        env!("CARGO_PKG_VERSION"),
        state.service_start_timestamp.format("%Y-%m-%d %H:%M:%S"),
    );

    let response_json = serde_json::json!({
        "status": "success".to_string(),
        "message": message,
    });
    Json(response_json)
}