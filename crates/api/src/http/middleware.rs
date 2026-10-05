// crates/api/src/http/middleware.rs
//! Middleware для аутентификации и извлечения контекста из JWT.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use rust_lms_shared::{IdentityId, TenantId};

use crate::auth::{JwtClaims, JwtManager};

/// Извлекает Bearer-токен, валидирует его и добавляет `IdentityId` и `TenantId` в extensions.
pub async fn jwt_auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;
    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let jwt_manager = req
        .extensions()
        .get::<JwtManager>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();

    let claims: JwtClaims = jwt_manager.validate_token(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    if claims.token_type != crate::auth::TokenType::Access {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let identity_id: IdentityId = claims.identity_id().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let tenant_id: TenantId = claims.tenant_id().map_err(|_| StatusCode::UNAUTHORIZED)?;

    tracing::debug!(identity_id = %identity_id, tenant_id = %tenant_id, "JWT authenticated successfully");

    req.extensions_mut().insert(identity_id);
    req.extensions_mut().insert(tenant_id);

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request as HttpRequest, routing::get, Router};
    use rust_lms_shared::IdentityId;
    use tower::ServiceExt;
    use crate::auth::{JwtConfig, JwtManager};

    async fn check_extensions(
        axum::extract::Extension(identity_id): axum::extract::Extension<IdentityId>,
        axum::extract::Extension(tenant_id): axum::extract::Extension<TenantId>,
    ) -> StatusCode {
        let _ = identity_id;
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
        let req = HttpRequest::builder().uri("/protected").body(Body::empty()).unwrap();
        assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_valid_access_token_accepted() {
        let jwt_manager = test_jwt_manager();
        let identity_id = IdentityId::new();
        let tenant_id = TenantId::new();
        let access_token = jwt_manager.generate_access_token(identity_id, tenant_id).unwrap();

        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::OK);
    }
}