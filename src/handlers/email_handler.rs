use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use crate::db;
use crate::models::{AppState, EmailConfig};

pub async fn get_email_config_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Option<EmailConfig>>, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB lock failed".to_string(),
        )
    })?;
    let config = db::get_email_config(&db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(mut c) = config {
        c.smtp_password = "".to_string();
        Ok(Json(Some(c)))
    } else {
        Ok(Json(None))
    }
}

pub async fn update_email_config_handler(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<EmailConfig>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB lock failed".to_string(),
        )
    })?;

    if payload.smtp_password.is_empty() {
        if let Ok(Some(existing)) = db::get_email_config(&db) {
            payload.smtp_password = existing.smtp_password;
        }
    }

    db::save_email_config(&db, &payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}