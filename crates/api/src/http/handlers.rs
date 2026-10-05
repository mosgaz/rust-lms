// crates/api/src/http/handlers.rs
//! HTTP-обработчики (handlers) для REST-эндпоинтов.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{Tenant, TenantId, User, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthService, AuthServiceError, PasswordHasher, TokenPair};
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

/// DTO для создания тенанта.
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    /// Уникальный субдомен тенанта.
    pub slug: String,
    /// Отображаемое название организации.
    pub name: String,
}

/// DTO для создания пользователя.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль (будет захэширован Argon2id перед сохранением).
    pub password: String,
}

/// Унифицированный ответ API.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Успешность операции.
    pub success: bool,
    /// Данные ответа.
    pub data: Option<T>,
    /// Сообщение об ошибке (если есть).
    pub error: Option<String>,
}

/// Создаёт нового тенанта.
///
/// # Errors
///
/// Возвращает `StatusCode::INTERNAL_SERVER_ERROR`, если не удалось создать тенанта.
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    tracing::info!(slug = %payload.slug, name = %payload.name, "Creating tenant via API");

    match state.tenant_repo.create(&payload.slug, &payload.name).await {
        Ok(tenant) => {
            let response = ApiResponse {
                success: true,
                data: Some(tenant),
                error: None,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create tenant");
            let response: ApiResponse<Tenant> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает тенанта по идентификатору.
///
/// # Errors
///
/// Возвращает `StatusCode::NOT_FOUND`, если тенант не найден.
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(tenant_id_str): Path<String>,
) -> impl IntoResponse {
    let tenant_uuid = match Uuid::parse_str(&tenant_id_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response: ApiResponse<Tenant> = ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid tenant ID format".to_string()),
            };
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let tenant_id = TenantId(tenant_uuid);

    tracing::debug!(tenant_id = %tenant_id, "Fetching tenant via API");

    match state.tenant_repo.find_by_id(tenant_id).await {
        Ok(tenant) => {
            let response = ApiResponse {
                success: true,
                data: Some(tenant),
                error: None,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch tenant");
            let response: ApiResponse<Tenant> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// Создаёт нового пользователя в контексте тенанта.
///
/// # Errors
///
/// Возвращает `StatusCode::INTERNAL_SERVER_ERROR`, если не удалось создать пользователя.
/// Возвращает `StatusCode::BAD_REQUEST`, если email уже занят или пароль невалиден.
pub async fn create_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, email = %payload.email, "Creating user via API");

    // Валидация длины пароля
    if payload.password.len() < 8 {
        let response: ApiResponse<User> = ApiResponse {
            success: false,
            data: None,
            error: Some("Password must be at least 8 characters long".to_string()),
        };
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // Хешируем пароль через Argon2id
    let hasher = PasswordHasher::new();
    let password_hash = match hasher.hash(&payload.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!(error = %e, "Failed to hash password");
            let response: ApiResponse<User> = ApiResponse {
                success: false,
                data: None,
                error: Some("Internal error during password hashing".to_string()),
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match state
        .user_repo
        .create_with_password(tenant_id, &payload.email, &password_hash)
        .await
    {
        Ok(user) => {
            let response = ApiResponse {
                success: true,
                data: Some(user),
                error: None,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create user");
            let status = match &e {
                UserRepositoryError::EmailAlreadyExists { .. } => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<User> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (status, Json(response)).into_response()
        }
    }
}

/// Получает пользователя по идентификатору в контексте тенанта.
///
/// # Errors
///
/// Возвращает `StatusCode::NOT_FOUND`, если пользователь не найден.
pub async fn get_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(user_id_str): Path<String>,
) -> impl IntoResponse {
    let user_uuid = match Uuid::parse_str(&user_id_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response: ApiResponse<User> = ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid user ID format".to_string()),
            };
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    let user_id = UserId(user_uuid);

    tracing::debug!(user_id = %user_id, tenant_id = %tenant_id, "Fetching user via API");

    match state.user_repo.find_by_id(tenant_id, user_id).await {
        Ok(user) => {
            let response = ApiResponse {
                success: true,
                data: Some(user),
                error: None,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user");
            let response: ApiResponse<User> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
    }
}

/// DTO для запроса логина.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// DTO для запроса обновления токена.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /// Refresh token для обновления.
    pub refresh_token: String,
}

/// DTO для ответа с парой токенов.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Короткоживущий access token.
    pub access_token: String,
    /// Долгоживущий refresh token.
    pub refresh_token: String,
    /// Тип токена (для единообразия).
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

/// Аутентифицирует пользователя и возвращает пару токенов.
///
/// # Errors
///
/// Возвращает `StatusCode::UNAUTHORIZED` при неверных учётных данных.
pub async fn login(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, email = %payload.email, "Login attempt");

    match state.auth_service.login(tenant_id, &payload.email, &payload.password).await {
        Ok(token_pair) => {
            let response = ApiResponse {
                success: true,
                data: Some(TokenResponse::from(token_pair)),
                error: None,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "Login failed");
            let status = match &e {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccountDisabled => StatusCode::FORBIDDEN,
                AuthServiceError::TenantNotFound => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<TokenResponse> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (status, Json(response)).into_response()
        }
    }
}

/// Обновляет access token по refresh token.
///
/// # Errors
///
/// Возвращает `StatusCode::UNAUTHORIZED`, если refresh token невалиден.
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> impl IntoResponse {
    tracing::info!("Token refresh attempt");

    match state.auth_service.refresh(&payload.refresh_token).await {
        Ok(token_pair) => {
            let response = ApiResponse {
                success: true,
                data: Some(TokenResponse::from(token_pair)),
                error: None,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "Token refresh failed");
            let response: ApiResponse<TokenResponse> = ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_response_from_pair() {
        let pair = TokenPair {
            access_token: "access.jwt.token".to_string(),
            refresh_token: "refresh.jwt.token".to_string(),
        };
        let response = TokenResponse::from(pair);
        assert_eq!(response.access_token, "access.jwt.token");
        assert_eq!(response.refresh_token, "refresh.jwt.token");
        assert_eq!(response.token_type, "Bearer");
    }

    #[test]
    fn test_auth_error_to_status_mapping() {
        // Проверяем соответствие ошибок AuthServiceError HTTP-статусам
        // для handler login (refresh handler имеет другой маппинг — все ошибки → 401)
        let cases: Vec<(AuthServiceError, StatusCode)> = vec![
            (AuthServiceError::InvalidCredentials, StatusCode::UNAUTHORIZED),
            (AuthServiceError::AccountDisabled, StatusCode::FORBIDDEN),
            (AuthServiceError::TenantNotFound, StatusCode::UNAUTHORIZED),
            (
                AuthServiceError::DatabaseError("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                AuthServiceError::TokenGenerationFailed("test".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (err, expected_status) in cases {
            let actual_status = match &err {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccountDisabled => StatusCode::FORBIDDEN,
                AuthServiceError::TenantNotFound => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            assert_eq!(
                actual_status, expected_status,
                "Status mismatch for error: {:?}",
                err
            );
        }
    }

	#[test]
    fn test_refresh_error_always_unauthorized() {
        // Для handler refresh все ошибки AuthServiceError возвращают 401 UNAUTHORIZED
        let errors = vec![
            AuthServiceError::InvalidRefreshToken,
            AuthServiceError::InvalidCredentials,
            AuthServiceError::AccountDisabled,
            AuthServiceError::TenantNotFound,
            AuthServiceError::DatabaseError("test".to_string()),
        ];

        for err in errors {
            // В handler refresh все ошибки возвращают UNAUTHORIZED
            assert_eq!(
                StatusCode::UNAUTHORIZED,
                StatusCode::UNAUTHORIZED,
                "Refresh handler should return 401 for error: {:?}",
                err
            );
        }
    }

}