// crates/api/src/http/router.rs
//! Конфигурация Axum-роутера.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::auth::{AuthService, JwtConfig, JwtManager};
use crate::database::{DatabasePool, TenantRepository, UserRepository};

use super::handlers::{self, AppState};
use super::middleware::jwt_auth;

/// Создаёт и настраивает Axum-роутер с middleware и эндпоинтами.
///
/// # Arguments
///
/// * `pool` - Пул соединений с базой данных.
/// * `jwt_config` - Конфигурация JWT (секрет, TTL).
///
/// # Returns
///
/// Настроенный `Router` с зарегистрированными маршрутами.
#[must_use]
pub fn create_router(pool: DatabasePool, jwt_config: JwtConfig) -> Router {
    let tenant_repo = TenantRepository::new(pool.inner().clone());
    let user_repo = UserRepository::new(pool.inner().clone());
    let jwt_manager = JwtManager::new(jwt_config);
    let auth_service = AuthService::new(user_repo.clone(), tenant_repo.clone(), jwt_manager.clone());

    let state = AppState {
        tenant_repo,
        user_repo,
        auth_service,
    };

    // Публичные эндпоинты (без аутентификации)
    let public_routes = Router::new()
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/refresh", post(handlers::refresh))
        .route("/api/v1/tenants", post(handlers::create_tenant))
        .route("/api/v1/tenants/:id", get(handlers::get_tenant));

    // Защищённые эндпоинты (требуют валидный JWT)
    let protected_routes = Router::new()
        .route("/api/v1/users", post(handlers::create_user))
        .route("/api/v1/users/:id", get(handlers::get_user))
        .layer(middleware::from_fn(jwt_auth));

    // JwtManager доступен для middleware через extensions
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(axum::Extension(jwt_manager))
}