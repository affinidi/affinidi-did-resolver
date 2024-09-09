use crate::{common::create_session_id, errors::ErrorResponse, SharedData};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use std::{
    fmt::{Debug, Display},
    net::SocketAddr,
};
use tracing::{info, warn};

#[derive(Debug)]
pub enum SessionError {
    SessionError(String),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::SessionError(message) => {
                write!(f, "Error: {}", message)
            }
        }
    }
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        let status = match self {
            SessionError::SessionError(_) => StatusCode::BAD_REQUEST,
        };
        let body = Json(json!(ErrorResponse {
            sessionId: "UNAUTHORIZED".into(),
            httpCode: status.as_u16(),
            errorCode: status.as_u16(),
            errorCodeStr: status.to_string(),
            message: self.to_string(),
        }));
        (status, body).into_response()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub session_id: String,  // Unique session transaction ID
    pub remote_addr: String, // Remote Socket address
}

#[async_trait]
impl<S> FromRequestParts<S> for Session
where
    SharedData: FromRef<S>,
    S: Send + Sync + Debug,
{
    type Rejection = SessionError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let remote_addr = if let Some(address) = parts
            .extensions
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0)
        {
            address.to_string()
        } else {
            warn!("No remote address in request!");
            return Err(SessionError::SessionError(
                "No remote address in request!".into(),
            ));
        };

        let session_id = create_session_id();

        info!(
            "{}: Connection accepted from address({})",
            &session_id, &remote_addr
        );

        let session = Session {
            session_id,
            remote_addr,
        };

        Ok(session)
    }
}
