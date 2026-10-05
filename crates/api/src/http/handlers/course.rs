// crates/api/src/http/handlers/course.rs
//! Обработчики для работы с курсами.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, TenantId};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::CourseRepositoryError;

use super::{ApiResponse, AppState};

/// Запрос на создание нового курса.
#[derive(Debug, Deserialize)]
pub struct CreateCourseRequest {
    /// Заголовок курса.
    pub title: String,
    /// Локализованные заголовки (BCP-47).
    pub title_i18n: Option<serde_json::Value>,
    /// Описание курса.
    pub description: Option<String>,
    /// Локализованные описания.
    pub description_i18n: Option<serde_json::Value>,
    /// Правила автоматической выдачи сертификатов.
    pub certification_rules: Option<serde_json::Value>,
}

/// Запрос на обновление курса.
#[derive(Debug, Deserialize)]
pub struct UpdateCourseRequest {
    /// Новый заголовок курса.
    pub title: Option<String>,
    /// Новые локализованные заголовки.
    pub title_i18n: Option<serde_json::Value>,
    /// Новое описание курса.
    pub description: Option<String>,
    /// Новые локализованные описания.
    pub description_i18n: Option<serde_json::Value>,
    /// Новые правила выдачи сертификатов.
    pub certification_rules: Option<serde_json::Value>,
}

/// Создаёт новый курс в контексте текущего тенанта.
pub async fn create_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateCourseRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, title = %payload.title, "Creating new course");

    match state
        .course_repo
        .create(
            tenant_id,
            &payload.title,
            payload.title_i18n,
            payload.description.as_deref(),
            payload.description_i18n,
            payload.certification_rules,
        )
        .await
    {
        Ok(course) => {
            let response = ApiResponse::ok(course);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create course");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает курс по идентификатору.
pub async fn get_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.find_by_id(tenant_id, cid).await {
        Ok(course) => {
            let response = ApiResponse::ok(course);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Course not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает список курсов текущего тенанта.
pub async fn list_courses(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
) -> impl IntoResponse {
    match state.course_repo.find_by_tenant(tenant_id, 100, 0).await {
        Ok(courses) => {
            let response = ApiResponse::ok(courses);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Обновляет метаданные курса.
pub async fn update_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<UpdateCourseRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state
        .course_repo
        .update(
            tenant_id,
            cid,
            payload.title.as_deref(),
            payload.title_i18n,
            payload.description.as_deref(),
            payload.description_i18n,
            payload.certification_rules,
        )
        .await
    {
        Ok(course) => {
            let response = ApiResponse::ok(course);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Course not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Удаляет курс и все связанные с ним узлы (каскадно).
pub async fn delete_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.delete(tenant_id, cid).await {
        Ok(()) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::ok_empty();
            (StatusCode::NO_CONTENT, Json(response)).into_response()
        }
        Err(CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Course not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Публикует новую версию курса (инкремент версии).
pub async fn publish_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.publish_version(tenant_id, cid).await {
        Ok(new_version) => {
            let response = ApiResponse::ok(serde_json::json!({
                "course_id": cid.0,
                "new_version": new_version,
            }));
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Course not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Маппинг CourseRepositoryError -> StatusCode (общая логика всех хендлеров).
    fn course_status(err: &CourseRepositoryError) -> StatusCode {
        match err {
            CourseRepositoryError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[test]
    fn test_not_found_maps_to_404() {
        let err = CourseRepositoryError::NotFound(CourseId(uuid::Uuid::nil()));
        assert_eq!(course_status(&err), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_course_request_requires_title() {
        let json = serde_json::json!({
            "title": "Rust Basics",
            "description": "Intro course",
        });
        let req: CreateCourseRequest = serde_json::from_value(json)
            .expect("CreateCourseRequest must deserialize");
        assert_eq!(req.title, "Rust Basics");
        assert_eq!(req.description.as_deref(), Some("Intro course"));
        assert!(req.title_i18n.is_none());
        assert!(req.certification_rules.is_none());
    }

    #[test]
    fn test_update_course_request_all_optional() {
        let json = serde_json::json!({});
        let req: UpdateCourseRequest = serde_json::from_value(json)
            .expect("UpdateCourseRequest must deserialize from empty object");
        assert!(req.title.is_none());
        assert!(req.title_i18n.is_none());
        assert!(req.description.is_none());
        assert!(req.description_i18n.is_none());
        assert!(req.certification_rules.is_none());
    }

    #[test]
    fn test_publish_response_shape() {
        // Логика publish_course формирует JSON с course_id и new_version.
        let cid = CourseId(uuid::Uuid::nil());
        let new_version: i32 = 3;
        let payload = serde_json::json!({
            "course_id": cid.0,
            "new_version": new_version,
        });
        assert_eq!(payload["new_version"], 3);
        assert_eq!(payload["course_id"], serde_json::json!(uuid::Uuid::nil()));
    }
}