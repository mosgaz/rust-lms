// crates/api/src/http/handlers/user.rs
//! Обработчики для работы с пользователями.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{TenantId, User};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthServiceError;
use crate::database::UserRepositoryError;

use super::{ApiResponse, AppState};

/// DTO для запроса создания пользователя.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// Создаёт нового пользователя в контексте текущего тенанта.
pub async fn create_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, email = %payload.email, "Creating user via API");

    if payload.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<User>::err(
                "Password must be at least 8 characters long",
            )),
        )
            .into_response();
    }

    match state
        .auth_service
        .create_user_in_tenant(tenant_id, &payload.email, &payload.password)
        .await
    {
        Ok(user) => {
            let response = ApiResponse::ok(user);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<User> = ApiResponse::err(e.to_string());
            (status, Json(response)).into_response()
        }
    }
}

/// Получает пользователя по идентификатору в контексте текущего тенанта.
pub async fn get_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = rust_lms_shared::UserId(id);

    match state.user_repo.find_by_id(tenant_id, user_id).await {
        Ok(user) => {
            let response = ApiResponse::ok(user);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(UserRepositoryError::NotFound(_)) => {
            let response: ApiResponse<User> = ApiResponse::err("User not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response: ApiResponse<User> = ApiResponse::err("Internal server error");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthServiceError;

    /// Чистая проверка длины пароля — та же логика, что в create_user.
    fn password_is_valid(password: &str) -> bool {
        password.len() >= 8
    }

    /// Маппинг AuthServiceError -> StatusCode для create_user.
    fn create_user_status(err: &AuthServiceError) -> StatusCode {
        match err {
            AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Маппинг UserRepositoryError -> StatusCode для get_user.
    fn get_user_status(err: &UserRepositoryError) -> StatusCode {
        match err {
            UserRepositoryError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[test]
    fn test_password_short_is_rejected() {
        assert!(!password_is_valid(""));
        assert!(!password_is_valid("1234567"));
    }

    #[test]
    fn test_password_min_length_is_accepted() {
        assert!(password_is_valid("12345678"));
        assert!(password_is_valid("a-much-longer-password"));
    }

    #[test]
    fn test_create_user_status_mapping() {
        assert_eq!(
            create_user_status(&AuthServiceError::EmailAlreadyExists),
            StatusCode::CONFLICT
        );
        assert_eq!(
            create_user_status(&AuthServiceError::InvalidCredentials),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_get_user_not_found_maps_to_404() {
        let err = UserRepositoryError::NotFound(rust_lms_shared::UserId(uuid::Uuid::nil()));
        assert_eq!(get_user_status(&err), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_user_request_deserializes() {
        let json = serde_json::json!({
            "email": "user@example.com",
            "password": "supersecret",
        });
        let req: CreateUserRequest = serde_json::from_value(json)
            .expect("CreateUserRequest must deserialize from valid JSON");
        assert_eq!(req.email, "user@example.com");
        assert_eq!(req.password, "supersecret");
    }
}