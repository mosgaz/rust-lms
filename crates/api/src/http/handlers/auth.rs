// crates/api/src/http/handlers/auth.rs
//! Обработчики аутентификации: login, select_tenant, refresh.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use rust_lms_shared::TenantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthServiceError, TokenPair};

use super::{ApiResponse, AppState};

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
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    tracing::info!(email = %payload.email, "Login attempt");

    match state
        .auth_service
        .authenticate(&payload.email, &payload.password)
        .await
    {
        Ok(crate::auth::AuthResult::SingleTenant(token_pair)) => {
            let response = ApiResponse::ok(TokenResponse::from(token_pair));
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(crate::auth::AuthResult::MultiTenant {
            session_token,
            available_tenants,
        }) => {
            let response = ApiResponse::ok(serde_json::json!({
                "session_token": session_token,
                "available_tenants": available_tenants,
            }));
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccessDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (status, Json(response)).into_response()
        }
    }
}

/// Выбирает конкретный тенант и выдаёт финальные токены.
pub async fn select_tenant(
    State(state): State<AppState>,
    Json(payload): Json<SelectTenantRequest>,
) -> impl IntoResponse {
    let tenant_uuid = match Uuid::parse_str(&payload.tenant_id) {
        Ok(uuid) => TenantId(uuid),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::err(
                    "Invalid tenant ID format",
                )),
            )
                .into_response();
        }
    };

    match state
        .auth_service
        .select_tenant(&payload.session_token, tenant_uuid)
        .await
    {
        Ok(token_pair) => {
            let response = ApiResponse::ok(TokenResponse::from(token_pair));
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidSessionToken | AuthServiceError::AccessDenied => {
                    StatusCode::UNAUTHORIZED
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (status, Json(response)).into_response()
        }
    }
}

/// Обновляет access token по валидному refresh token.
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let refresh_token = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match state.auth_service.refresh(refresh_token).await {
        Ok(token_pair) => {
            let response = ApiResponse::ok(TokenResponse::from(token_pair));
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthServiceError;

    #[test]
    fn test_token_response_from_pair() {
        let pair = TokenPair {
            access_token: "a".to_string(),
            refresh_token: "r".to_string(),
        };
        let response = TokenResponse::from(pair);
        assert_eq!(response.access_token, "a");
        assert_eq!(response.refresh_token, "r");
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