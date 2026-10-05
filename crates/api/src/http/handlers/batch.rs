// crates/api/src/http/handlers/batch.rs
//! Обработчики для работы с потоками (Batches).

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_lms_shared::{BatchId, BatchStatus, TenantId};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::BatchRepositoryError;

use super::{ApiResponse, AppState};

/// Запрос на создание потока.
#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    /// Название потока.
    pub title: String,
    /// Описание потока.
    pub description: Option<String>,
    /// Статус потока (по умолчанию draft).
    pub status: Option<BatchStatus>,
    /// Дата начала обучения.
    pub start_date: Option<DateTime<Utc>>,
    /// Дата окончания обучения.
    pub end_date: Option<DateTime<Utc>>,
    /// Дедлайн для зачисления.
    pub enrollment_deadline: Option<DateTime<Utc>>,
}

/// Запрос на обновление потока.
#[derive(Debug, Deserialize)]
pub struct UpdateBatchRequest {
    /// Новое название.
    pub title: Option<String>,
    /// Новое описание.
    pub description: Option<String>,
    /// Новый статус.
    pub status: Option<BatchStatus>,
    /// Новая дата начала.
    pub start_date: Option<DateTime<Utc>>,
    /// Новая дата окончания.
    pub end_date: Option<DateTime<Utc>>,
    /// Новый дедлайн зачисления.
    pub enrollment_deadline: Option<DateTime<Utc>>,
}

/// Создаёт новый поток.
pub async fn create_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateBatchRequest>,
) -> impl IntoResponse {
    match state
        .batch_repo
        .create(
            tenant_id,
            &payload.title,
            payload.description.as_deref(),
            payload.status.unwrap_or(BatchStatus::Draft),
            payload.start_date,
            payload.end_date,
            payload.enrollment_deadline,
        )
        .await
    {
        Ok(batch) => (StatusCode::CREATED, Json(ApiResponse::ok(batch))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Получает список потоков тенанта.
pub async fn list_batches(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
) -> impl IntoResponse {
    match state.batch_repo.find_by_tenant(tenant_id, 100, 0).await {
        Ok(batches) => (StatusCode::OK, Json(ApiResponse::ok(batches))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Получает поток по идентификатору.
pub async fn get_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(batch_id): Path<Uuid>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    match state.batch_repo.find_by_id(tenant_id, bid).await {
        Ok(batch) => (StatusCode::OK, Json(ApiResponse::ok(batch))).into_response(),
        Err(BatchRepositoryError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err("Batch not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Обновляет метаданные потока.
pub async fn update_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(batch_id): Path<Uuid>,
    Json(payload): Json<UpdateBatchRequest>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    match state
        .batch_repo
        .update(
            tenant_id,
            bid,
            payload.title.as_deref(),
            payload.description.as_deref(),
            payload.status,
            payload.start_date,
            payload.end_date,
            payload.enrollment_deadline,
        )
        .await
    {
        Ok(batch) => (StatusCode::OK, Json(ApiResponse::ok(batch))).into_response(),
        Err(BatchRepositoryError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err("Batch not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Удаляет поток (каскадно).
pub async fn delete_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(batch_id): Path<Uuid>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    match state.batch_repo.delete(tenant_id, bid).await {
        Ok(()) => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<serde_json::Value>::ok_empty()),
        )
            .into_response(),
        Err(BatchRepositoryError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err("Batch not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}