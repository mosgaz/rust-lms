// crates/api/src/http/handlers/course_enrollment.rs
//! Обработчики для работы с индивидуальными зачислениями на курсы.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, TenantId, UserId};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::CourseEnrollmentRepositoryError;

use super::{ApiResponse, AppState};

/// Запрос на зачисление пользователя на курс.
#[derive(Debug, Deserialize)]
pub struct EnrollToCourseRequest {
    /// Идентификатор пользователя.
    pub user_id: Uuid,
}

/// Зачисляет пользователя на курс (self-paced).
pub async fn enroll_to_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<EnrollToCourseRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    let uid = UserId(payload.user_id);

    match state
        .course_enrollment_repo
        .enroll(cid, uid, tenant_id)
        .await
    {
        Ok(enrollment) => (StatusCode::CREATED, Json(ApiResponse::ok(enrollment))).into_response(),
        Err(CourseEnrollmentRepositoryError::AlreadyEnrolled { .. }) => (
            StatusCode::CONFLICT,
            Json(ApiResponse::<serde_json::Value>::err(
                "User is already enrolled in this course",
            )),
        )
            .into_response(),
        Err(CourseEnrollmentRepositoryError::CourseNotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err("Course not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        )
            .into_response(),
    }
}

/// Отчисляет пользователя с курса (soft delete).
pub async fn unenroll_from_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path((course_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    let uid = UserId(user_id);

    match state
        .course_enrollment_repo
        .unenroll(cid, uid, tenant_id)
        .await
    {
        Ok(()) => (
            StatusCode::NO_CONTENT,
            Json(ApiResponse::<serde_json::Value>::ok_empty()),
        )
            .into_response(),
        Err(CourseEnrollmentRepositoryError::NotFound) => (
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

/// Получает список студентов курса.
pub async fn list_course_enrollments(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state
        .course_enrollment_repo
        .find_by_course(cid, tenant_id)
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

/// Получает все курсы, на которые зачислен пользователь.
pub async fn list_user_course_enrollments(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let uid = UserId(user_id);
    match state
        .course_enrollment_repo
        .find_by_user(uid, tenant_id)
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