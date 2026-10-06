// crates/api/src/http/handlers/lesson_progress.rs
//! Обработчики для управления прогрессом обучения.

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{ProgressUpdateRequest, ProgressResponse, TenantId, UserId};

use crate::database::LessonProgressRepositoryError;

use super::{ApiResponse, AppState};

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
            let response = ApiResponse::ok(ProgressResponse {
                lesson_progress: result.lesson_progress,
                course_progress: result.course_progress,
                course_status: result.course_status,
                completion_triggered: result.completion_triggered,
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let (status, msg) = match e {
                LessonProgressRepositoryError::NodeNotFound => {
                    (StatusCode::NOT_FOUND, "Node not found".to_string())
                }
                LessonProgressRepositoryError::NodeArchived => {
                    (StatusCode::GONE, "Node is archived".to_string())
                }
                LessonProgressRepositoryError::NotEnrolled => {
                    (StatusCode::FORBIDDEN, "User not enrolled in this course".to_string())
                }
                LessonProgressRepositoryError::CourseAlreadyCompleted => {
                    (StatusCode::CONFLICT, "Course already completed".to_string())
                }
                LessonProgressRepositoryError::Database(db_err) => {
                    tracing::error!(error = %db_err, "Database error updating progress");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
                }
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(msg);
            (status, Json(response)).into_response()
        }
    }
}