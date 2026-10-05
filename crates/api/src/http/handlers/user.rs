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
    // Тесты на валидацию пароля можно добавить здесь, вынеся проверку в чистую функцию.
}