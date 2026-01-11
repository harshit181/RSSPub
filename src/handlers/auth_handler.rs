use axum::http::{header, StatusCode};
use base64::Engine;
use axum::response::IntoResponse;
use crate::routes::{RPUB_PASSWORD, RPUB_USERNAME};

pub async fn auth(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let username = std::env::var(RPUB_USERNAME).unwrap_or_default();
    let password = std::env::var(RPUB_PASSWORD).unwrap_or_default();

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Basic ") {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(token) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    if let Some((u, p)) = credentials.split_once(':') {
                        if u == username && p == password {
                            return next.run(req).await.into_response();
                        }
                    }
                }
            }
        }
    }

    (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into_response()
}