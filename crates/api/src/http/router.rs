// crates/api/src/http/router.rs
//! Конфигурация Axum-роутера.

use axum::{middleware, routing::{get, post}, Router};
use sqlx::PgPool;

use crate::auth::{AuthService, JwtConfig, JwtManager};
use crate::database::{IdentityRepository, TenantRepository, UserRepository};

use super::handlers::{self, AppState};
use super::middleware::jwt_auth;

/// Создаёт и настраивает Axum-роутер с middleware и эндпоинтами.
#[must_use]
pub fn create_router(pool: PgPool, jwt_config: JwtConfig) -> Router {
    let identity_repo = IdentityRepository::new(pool.clone());
    let user_repo = UserRepository::new(pool.clone());
    let tenant_repo = TenantRepository::new(pool.clone());
    let jwt_manager = JwtManager::new(jwt_config);

    let auth_service = AuthService::new(
        identity_repo,
        user_repo.clone(),
        tenant_repo.clone(),
        jwt_manager.clone(),
    );

    let state = AppState {
        tenant_repo,
        user_repo,
        auth_service,
    };

    let public_routes = Router::new()
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/select-tenant", post(handlers::select_tenant))
        .route("/api/v1/auth/refresh", post(handlers::refresh))
        .route("/api/v1/tenants", post(handlers::create_tenant))
        .route("/api/v1/tenants/:id", get(handlers::get_tenant));

    let protected_routes = Router::new()
        .route("/api/v1/users", post(handlers::create_user))
        .route("/api/v1/users/:id", get(handlers::get_user))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(axum::Extension(jwt_manager))
}