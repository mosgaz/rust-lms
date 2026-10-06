// crates/api/src/http/handlers/lesson_progress.rs
//! Обработчики для управления прогрессом обучения.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, ProgressUpdateRequest, ProgressResponse, TenantId, UserId};
use serde::Deserialize;

use crate::database::LessonProgressRepositoryError;

use super::{ApiResponse, AppState};

/// Параметры пагинации для инструкторских эндпоинтов.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Максимальное количество записей (по умолчанию 100).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Смещение от начала списка (по умолчанию 0).
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Обновляет прогресс урока и автоматически пересчитывает прогресс курса.
pub async fn update_lesson_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Json(payload): Json<ProgressUpdateRequest>,
) -> impl IntoResponse {
    match state
        .progress_service
        .update_lesson_progress(
            tenant_id,
            user_id,
            payload.node_id,
            payload.status,
            payload.score,
            payload.time_spent_seconds,
            payload.last_position,
            payload.client_modified_at,
        )
        .await
    {
        Ok(result) => (
            StatusCode::OK,
            Json(ApiResponse::ok(ProgressResponse {
                lesson_progress: result.lesson_progress,
                course_progress: result.course_progress,
                course_status: result.course_status,
                completion_triggered: result.completion_triggered,
            })),
        )
            .into_response(),
        Err(e) => map_error_to_response(e),
    }
}

/// Получает детальный прогресс пользователя по конкретному курсу (все уроки, включая не начатые).
pub async fn get_user_course_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(course_id): Path<CourseId>,
) -> impl IntoResponse {
    match state
        .progress_service
        .get_user_course_progress(tenant_id, user_id, course_id)
        .await
    {
        Ok(progress) => (StatusCode::OK, Json(ApiResponse::ok(progress))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Получает прогресс всех студентов курса (для инструктора, с пагинацией).
pub async fn get_course_students_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<CourseId>,
    Query(pagination): Query<PaginationQuery>,
) -> impl IntoResponse {
    match state
        .progress_service
        .get_course_students_progress(
            tenant_id,
            course_id,
            pagination.limit,
            pagination.offset,
        )
        .await
    {
        Ok(progress) => (StatusCode::OK, Json(ApiResponse::ok(progress))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Принудительный пересчёт прогресса курса (для инструктора/админа).
pub async fn recalculate_course_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<CourseId>,
) -> impl IntoResponse {
    match state
        .progress_service
        .recalculate_course_progress(tenant_id, course_id)
        .await
    {
        Ok(rows_affected) => (StatusCode::OK, Json(ApiResponse::ok(rows_affected))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Преобразует ошибку репозитория в HTTP-ответ с соответствующим статус-кодом.
fn map_error_to_response(e: LessonProgressRepositoryError) -> axum::response::Response {
    let (status, msg) = match e {
        LessonProgressRepositoryError::NodeNotFound => {
            (StatusCode::NOT_FOUND, "Node not found".to_string())
        }
        LessonProgressRepositoryError::NodeArchived => {
            (StatusCode::GONE, "Node is archived".to_string())
        }
        LessonProgressRepositoryError::NotEnrolled => (
            StatusCode::FORBIDDEN,
            "User not enrolled in this course".to_string(),
        ),
        LessonProgressRepositoryError::CourseAlreadyCompleted => (
            StatusCode::CONFLICT,
            "Course already completed".to_string(),
        ),
        LessonProgressRepositoryError::Database(db_err) => {
            tracing::error!(error = %db_err, "Database error updating progress");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        }
    };
    (status, Json(ApiResponse::<serde_json::Value>::err(msg))).into_response()
}