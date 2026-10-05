// crates/api/src/http/middleware.rs
//! Middleware для аутентификации и извлечения контекста тенанта из JWT.
//!
//! Извлекает Bearer-токен из заголовка `Authorization`, валидирует его через
//! `JwtManager` и добавляет `TenantId` и `UserId` в `Request::extensions`
//! для последующего использования в handlers и RLS-интерцепторе.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use rust_lms_shared::{TenantId, UserId};

use crate::auth::{JwtClaims, JwtManager};

/// Извлекает Bearer-токен из заголовка `Authorization`, валидирует его
/// и добавляет `TenantId` и `UserId` в расширения запроса.
///
/// # Errors
///
/// Возвращает `StatusCode::UNAUTHORIZED`, если токен отсутствует, невалиден или истёк.
pub async fn jwt_auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    // 1. Извлекаем заголовок Authorization
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. Проверяем формат "Bearer <token>"
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 3. Получаем JwtManager из extensions (устанавливается при создании роутера)
    let jwt_manager = req
        .extensions()
        .get::<JwtManager>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();

    // 4. Валидируем токен
    let claims: JwtClaims = jwt_manager
        .validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 5. Проверяем, что это access token (refresh не подходит для авторизации)
    if claims.token_type != crate::auth::TokenType::Access {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 6. Извлекаем user_id и tenant_id
    let user_id: UserId = claims.user_id().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let tenant_id: TenantId = claims.tenant_id().map_err(|_| StatusCode::UNAUTHORIZED)?;

    tracing::debug!(
        user_id = %user_id,
        tenant_id = %tenant_id,
        "JWT authenticated successfully"
    );

    // 7. Добавляем в extensions для handlers
    req.extensions_mut().insert(user_id);
    req.extensions_mut().insert(tenant_id);

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request as HttpRequest, routing::get, Router};
    use tower::ServiceExt;

    use crate::auth::JwtConfig;

    /// Вспомогательный хендлер для проверки, что middleware корректно добавил extensions.
    async fn check_extensions(
        axum::extract::Extension(user_id): axum::extract::Extension<UserId>,
        axum::extract::Extension(tenant_id): axum::extract::Extension<TenantId>,
    ) -> StatusCode {
        let _ = user_id;
        let _ = tenant_id;
        StatusCode::OK
    }

    fn test_jwt_manager() -> JwtManager {
        JwtManager::new(JwtConfig {
            secret: "test-secret-that-is-at-least-32-bytes-long!".to_string(),
            access_ttl_secs: 900,
            refresh_ttl_secs: 604_800,
        })
    }

    fn test_app() -> Router {
        let jwt_manager = test_jwt_manager();
        Router::new()
            .route("/protected", get(check_extensions))
            .layer(axum::middleware::from_fn(jwt_auth))
            .layer(axum::Extension(jwt_manager))
    }

    #[tokio::test]
    async fn test_missing_authorization_header() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_bearer_format() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_empty_bearer_token() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, "Bearer ")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_jwt_token() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, "Bearer invalid.token.here")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_refresh_token_rejected() {
        let jwt_manager = test_jwt_manager();
        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        // Генерируем refresh token (должен быть отклонён)
        let refresh_token = jwt_manager
            .generate_refresh_token(user_id, tenant_id)
            .expect("token generation must succeed");

        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, format!("Bearer {}", refresh_token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_valid_access_token_accepted() {
        let jwt_manager = test_jwt_manager();
        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        // Генерируем access token
        let access_token = jwt_manager
            .generate_access_token(user_id, tenant_id)
            .expect("token generation must succeed");

        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}