// crates/api/src/http/handlers/lesson_progress.rs
//! Обработчики для управления прогрессом обучения.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, ProgressResponse, ProgressUpdateRequest, TenantId, UserId};
use serde::Deserialize;

use crate::database::{LessonProgressRepositoryError, RlsContext}; // <-- ДОБАВЛЕНО: RlsContext

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
    tracing::info!(
        user_id = %user_id,
        node_id = %payload.node_id,
        "Updating lesson progress"
    );

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
        Ok(result) => {
            // =================================================================
            // === ЭТАП 11.5: Автоматическая выдача сертификата при завершении курса ===
            // =================================================================
            if result.completion_triggered {
                // ИСПРАВЛЕНО: RlsContext::new принимает только tenant_id
                let ctx = RlsContext::new(tenant_id);
                let course_uuid = result.course_id.0;

                tracing::info!(
                    user_id = %user_id,
                    course_id = %course_uuid,
                    "Course completed, triggering automatic certificate issuance"
                );

                // Graceful degradation: ошибки логируются, но не прерывают прогресс
                if let Err(e) = state
                    .certificate_service
                    .issue_course_certificate(&ctx, user_id.0, course_uuid)
                    .await
                {
                    tracing::error!(
                        error = %e,
                        user_id = %user_id,
                        course_id = %course_uuid,
                        "Failed to issue certificate after course completion"
                    );
                }
            }
            // =================================================================

            (
                StatusCode::OK,
                Json(ApiResponse::ok(ProgressResponse {
                    lesson_progress: result.lesson_progress,
                    course_progress: result.course_progress,
                    course_status: result.course_status,
                    completion_triggered: result.completion_triggered,
                })),
            )
                .into_response()
        }
        Err(e) => map_error_to_response(e),
    }
}

/// Получает детальный прогресс пользователя по конкретному курсу.
///
/// `GET /api/v1/progress/me/course/{course_id}`
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
        Err(e) => map_error_to_response(e),
    }
}

/// Получает прогресс всех студентов курса (для инструктора/админа).
///
/// `GET /api/v1/courses/{course_id}/progress`
pub async fn get_course_students_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(course_id): Path<CourseId>,
    Query(pagination): Query<PaginationQuery>,
) -> impl IntoResponse {
    let is_allowed = match state
        .progress_service
        .is_instructor_or_admin(tenant_id, user_id)
        .await
    {
        Ok(allowed) => allowed,
        Err(_) => false,
    };

    if !is_allowed {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::err("Insufficient permissions")),
        )
            .into_response();
    }

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
        Err(e) => map_error_to_response(e),
    }
}

/// Принудительный пересчёт прогресса для всех зачисленных студентов курса.
///
/// # Safety
///
/// Эта операция блокирует все зачисления курса (`FOR UPDATE OF ce`) на время выполнения.
/// Для больших курсов (1000+ студентов) это может занять несколько секунд.
/// Рекомендуется запускать в нерабочее время или через background worker.
///
/// `POST /api/v1/courses/{course_id}/progress/recalculate`
pub async fn recalculate_course_progress(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(course_id): Path<CourseId>,
) -> impl IntoResponse {
    let is_allowed = match state
        .progress_service
        .is_instructor_or_admin(tenant_id, user_id)
        .await
    {
        Ok(allowed) => allowed,
        Err(_) => false,
    };

    if !is_allowed {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::err("Insufficient permissions")),
        )
            .into_response();
    }

    match state
        .progress_service
        .recalculate_course_progress(tenant_id, course_id)
        .await
    {
        Ok(rows_affected) => (StatusCode::OK, Json(ApiResponse::ok(rows_affected))).into_response(),
        Err(e) => map_error_to_response(e),
    }
}

/// Маппинг ошибок репозитория прогресса в HTTP-ответы.
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
        LessonProgressRepositoryError::Forbidden => (
            StatusCode::FORBIDDEN,
            "Insufficient permissions".to_string(),
        ),
        LessonProgressRepositoryError::Database(db_err) => {
            tracing::error!(error = %db_err, "Database error in progress operation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        }
    };
    (status, Json(ApiResponse::<serde_json::Value>::err(msg))).into_response()
}