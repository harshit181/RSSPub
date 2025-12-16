use std::sync::Arc;
use axum::Json;
use axum::http::StatusCode;
use axum::extract::State;
use tracing::info;
use crate::models::{AppState, GenerateRequest};
use crate::{db, email, processor, util};

pub async fn list_downloads() -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(util::EPUB_OUTPUT_DIR)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read downloads: {}", e),
            )
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read entry: {}", e),
        )
    })? {
        if let Ok(name) = entry.file_name().into_string() {
            if name.ends_with(".epub") {
                files.push(name);
            }
        }
    }

    files.sort_by(|a, b| b.cmp(a));
    Ok(Json(files))
}

pub async fn generate_epub_adhoc(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    info!("Received request to generate EPUB");

    let feeds_to_fetch = if payload.feeds.is_empty() {
        let db = state.db.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB lock failed".to_string(),
            )
        })?;
        let stored_feeds =
            db::get_feeds(&db).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        stored_feeds
    } else {
        payload.feeds
    };

    if feeds_to_fetch.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No feeds provided and no stored feeds found.".to_string(),
        ));
    }

    let db_clone = state.db.clone();

    tokio::spawn(async move {
        info!("Starting background EPUB generation...");
        match processor::generate_and_save(feeds_to_fetch, &db_clone, util::EPUB_OUTPUT_DIR).await
        {
            Ok(filename) => {
                info!("Background generation completed successfully: {}", filename);
                match  email::check_and_send_email(db_clone, &filename).await {
                    Ok(_ok) => {}
                    Err(_error) => {}
                }
            }
            Err(e) => {
                tracing::error!("Failed to error: {}", e);
            }
        }
    });

    Ok(StatusCode::ACCEPTED)
}
