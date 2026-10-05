// crates/api/src/http/handlers.rs
//! REST-обработчики для API.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{Tenant, TenantId, User};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthService, AuthServiceError, TokenPair};
use crate::database::{TenantRepository, UserRepository, UserRepositoryError};

/// Состояние приложения, общее для всех handlers.
#[derive(Clone)]
pub struct AppState {
    /// Репозиторий для работы с тенантами.
    pub tenant_repo: TenantRepository,
    /// Репозиторий для работы с пользователями.
    pub user_repo: UserRepository,
    /// Сервис аутентификации.
    pub auth_service: AuthService,
}

/// Унифицированный формат ответа API.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Флаг успешности операции.
    pub success: bool,
    /// Полезные данные (при успехе).
    pub data: Option<T>,
    /// Сообщение об ошибке (при неудаче).
    pub error: Option<String>,
}

/// DTO для запроса логина.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// DTO для запроса выбора тенанта.
#[derive(Debug, Deserialize)]
pub struct SelectTenantRequest {
    /// Короткоживущий session token.
    pub session_token: String,
    /// Идентификатор выбранного тенанта.
    pub tenant_id: String,
}

/// DTO для запроса создания пользователя.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// DTO для ответа с парой токенов.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Короткоживущий access token.
    pub access_token: String,
    /// Долгоживущий refresh token.
    pub refresh_token: String,
    /// Тип токена.
    pub token_type: String,
}

impl From<TokenPair> for TokenResponse {
    fn from(pair: TokenPair) -> Self {
        Self {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            token_type: "Bearer".to_string(),
        }
    }
}

/// Аутентифицирует пользователя по email и паролю.
pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    tracing::info!(email = %payload.email, "Login attempt");

    match state.auth_service.authenticate(&payload.email, &payload.password).await {
        Ok(crate::auth::AuthResult::SingleTenant(token_pair)) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(crate::auth::AuthResult::MultiTenant { session_token, available_tenants }) => {
            let response = ApiResponse { 
                success: true, 
                data: Some(serde_json::json!({ "session_token": session_token, "available_tenants": available_tenants })), 
                error: None 
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccessDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (status, Json(response)).into_response()
        }
    }
}

/// Выбирает конкретный тенант и выдаёт финальные токены.
pub async fn select_tenant(State(state): State<AppState>, Json(payload): Json<SelectTenantRequest>) -> impl IntoResponse {
    let tenant_uuid = match Uuid::parse_str(&payload.tenant_id) {
        Ok(uuid) => TenantId(uuid),
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                success: false, data: None, error: Some("Invalid tenant ID format".to_string())
            })).into_response();
        }
    };

    match state.auth_service.select_tenant(&payload.session_token, tenant_uuid).await {
        Ok(token_pair) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidSessionToken | AuthServiceError::AccessDenied => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (status, Json(response)).into_response()
        }
    }
}

/// Обновляет access token по валидному refresh token.
pub async fn refresh(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let refresh_token = payload.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("");
    
    match state.auth_service.refresh(refresh_token).await {
        Ok(token_pair) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
    }
}

/// Создаёт нового тенанта (заглушка).
pub async fn create_tenant(State(_state): State<AppState>, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let slug = payload.get("slug").and_then(|v| v.as_str()).unwrap_or("default");
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Default Tenant");
    
    let mock_tenant = Tenant {
        id: TenantId(Uuid::new_v4()),
        slug: slug.to_string(),
        name: name.to_string(),
        is_active: true,
    };
    
    let response = ApiResponse { success: true, data: Some(mock_tenant), error: None };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Получает тенанта по идентификатору (заглушка).
pub async fn get_tenant(State(_state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let mock_tenant = Tenant {
        id: TenantId(id),
        slug: "mock-slug".to_string(),
        name: "Mock Tenant".to_string(),
        is_active: true,
    };
    let response = ApiResponse { success: true, data: Some(mock_tenant), error: None };
    (StatusCode::OK, Json(response)).into_response()
}

/// Создаёт нового пользователя в контексте текущего тенанта.
pub async fn create_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, email = %payload.email, "Creating user via API");

    if payload.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Json(ApiResponse::<User> {
            success: false, data: None, error: Some("Password must be at least 8 characters long".to_string())
        })).into_response();
    }

    match state.auth_service.create_user_in_tenant(tenant_id, &payload.email, &payload.password).await {
        Ok(user) => {
            let response = ApiResponse { success: true, data: Some(user), error: None };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
            let response = ApiResponse { success: true, data: Some(user), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(UserRepositoryError::NotFound(_)) => {
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some("User not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some("Internal server error".to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_response_from_pair() {
        let pair = TokenPair { access_token: "a".to_string(), refresh_token: "r".to_string() };
        let response = TokenResponse::from(pair);
        assert_eq!(response.access_token, "a");
        assert_eq!(response.token_type, "Bearer");
    }

    #[test]
    fn test_auth_error_to_status_mapping() {
        let cases: Vec<(AuthServiceError, StatusCode)> = vec![
            (AuthServiceError::InvalidCredentials, StatusCode::UNAUTHORIZED),
            (AuthServiceError::AccessDenied, StatusCode::FORBIDDEN),
            (AuthServiceError::EmailAlreadyExists, StatusCode::CONFLICT),
        ];

        for (err, expected) in cases {
            let actual = match &err {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccessDenied => StatusCode::FORBIDDEN,
                AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            assert_eq!(actual, expected, "Status mismatch for error: {:?}", err);
        }
    }
}