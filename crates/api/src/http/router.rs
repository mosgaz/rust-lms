// crates/api/src/http/router.rs
//! Конфигурация Axum-роутера.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::database::{DatabasePool, TenantRepository, UserRepository};

use super::handlers::{self, AppState};
use super::middleware::extract_tenant_context;

/// Создаёт и настраивает Axum-роутер с middleware и эндпоинтами.
///
/// # Arguments
///
/// * `pool` - Пул соединений с базой данных.
///
/// # Returns
///
/// Настроенный `Router` с зарегистрированными маршрутами.
#[must_use]
pub fn create_router(pool: DatabasePool) -> Router {
    let tenant_repo = TenantRepository::new(pool.inner().clone());
    let user_repo = UserRepository::new(pool.inner().clone());

    let state = AppState {
        tenant_repo,
        user_repo,
    };

    // Публичные эндпоинты (без контекста тенанта)
    let public_routes = Router::new()
        .route("/api/v1/tenants", post(handlers::create_tenant))
        .route("/api/v1/tenants/:id", get(handlers::get_tenant));

    // Защищённые эндпоинты (требуют X-Tenant-ID заголовок)
    let tenant_routes = Router::new()
        .route("/api/v1/users", post(handlers::create_user))
        .route("/api/v1/users/:id", get(handlers::get_user))
        .layer(middleware::from_fn(extract_tenant_context));

    Router::new()
        .merge(public_routes)
        .merge(tenant_routes)
        .with_state(state)
}