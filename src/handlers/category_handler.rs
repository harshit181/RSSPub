use crate::models::{AppState, Category, CreateCategoryRequest, ReorderCategoriesRequest};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use crate::db::category_db;

pub async fn list_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Category>>, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed".to_string()))?;
    let cats = category_db::get_categories(&db).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(cats))
}

pub async fn add_category(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed".to_string()))?;
    category_db::add_category(&db, &payload.name).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::CREATED)
}

pub async fn update_category(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed".to_string()))?;
    category_db::update_category(&db, id, &payload.name).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

pub async fn delete_category(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed".to_string()))?;
    category_db::delete_category(&db, id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder_categories(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReorderCategoriesRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed".to_string()))?;
    category_db::reorder_categories(&db, &payload.categories).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}
