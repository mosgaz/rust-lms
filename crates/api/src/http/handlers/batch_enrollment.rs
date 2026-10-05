// crates/api/src/http/handlers/batch_enrollment.rs
//! Обработчики для работы с зачислениями в потоки.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{BatchId, BatchRole, TenantId, UserId};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::BatchEnrollmentRepositoryError;

use super::{ApiResponse, AppState};

/// Запрос на зачисление пользователя в поток.
#[derive(Debug, Deserialize)]
pub struct EnrollToBatchRequest {
    /// Идентификатор пользователя.
    pub user_id: Uuid,
    /// Роль в потоке (по умолчанию student).
    pub role: Option<BatchRole>,
}

/// Запрос на изменение роли в потоке.
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    /// Новая роль.
    pub role: BatchRole,
}

/// Зачисляет пользователя в поток.
pub async fn enroll_to_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(batch_id): Path<Uuid>,
    Json(payload): Json<EnrollToBatchRequest>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    let uid = UserId(payload.user_id);
    let role = payload.role.unwrap_or(BatchRole::Student);

    match state
        .batch_enrollment_repo
        .enroll(bid, uid, role, tenant_id)
        .await
    {
        Ok(enrollment) => (StatusCode::CREATED, Json(ApiResponse::ok(enrollment))).into_response(),
        Err(BatchEnrollmentRepositoryError::AlreadyEnrolled { .. }) => (
            StatusCode::CONFLICT,
            Json(ApiResponse::<serde_json::Value>::err(
                "User is already enrolled in this batch",
            )),
        )
            .into_response(),
        Err(BatchEnrollmentRepositoryError::BatchNotFound(_)) => (
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

/// Отчисляет пользователя из потока (soft delete).
pub async fn unenroll_from_batch(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path((batch_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    let uid = UserId(user_id);

    match state
        .batch_enrollment_repo
        .unenroll(bid, uid, tenant_id)
        .await
    {
        Ok(()) => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<serde_json::Value>::ok_empty()),
        )
            .into_response(),
        Err(BatchEnrollmentRepositoryError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err(
                "Enrollment not found or already dropped",
            )),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Получает список участников потока.
pub async fn list_batch_enrollments(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(batch_id): Path<Uuid>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    match state
        .batch_enrollment_repo
        .find_by_batch(bid, tenant_id)
        .await
    {
        Ok(enrollments) => (StatusCode::OK, Json(ApiResponse::ok(enrollments))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Обновляет роль участника потока.
pub async fn update_batch_enrollment_role(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path((batch_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    let bid = BatchId(batch_id);
    let uid = UserId(user_id);

    match state
        .batch_enrollment_repo
        .update_role(bid, uid, payload.role, tenant_id)
        .await
    {
        Ok(enrollment) => (StatusCode::OK, Json(ApiResponse::ok(enrollment))).into_response(),
        Err(BatchEnrollmentRepositoryError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err("Enrollment not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}