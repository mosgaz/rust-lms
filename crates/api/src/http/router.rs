// crates/api/src/http/router.rs
//! Конфигурация Axum-роутера.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::auth::{AuthService, JwtConfig, JwtManager};
use crate::database::{
    CourseRepository, IdentityRepository, NodeRepository, TenantRepository, UserRepository,
};

use super::handlers::{self, AppState};
use super::middleware::jwt_auth;

/// Создаёт и настраивает Axum-роутер с middleware и эндпоинтами.
#[must_use]
pub fn create_router(pool: PgPool, jwt_config: JwtConfig) -> Router {
    let identity_repo = IdentityRepository::new(pool.clone());
    let user_repo = UserRepository::new(pool.clone());
    let tenant_repo = TenantRepository::new(pool.clone());
    let course_repo = CourseRepository::new(pool.clone());
    let node_repo = NodeRepository::new(pool.clone());
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
        course_repo,
        node_repo,
    };

    let public_routes = Router::new()
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/select-tenant", post(handlers::select_tenant))
        .route("/api/v1/auth/refresh", post(handlers::refresh))
        .route("/api/v1/tenants", post(handlers::create_tenant))
        .route("/api/v1/tenants/:id", get(handlers::get_tenant));

    let protected_routes = Router::new()
        // Users (tenant-scoped)
        .route("/api/v1/users", post(handlers::create_user))
        .route("/api/v1/users/:id", get(handlers::get_user))
        // Courses (tenant-scoped)
        .route(
            "/api/v1/courses",
            get(handlers::list_courses).post(handlers::create_course),
        )
        .route(
            "/api/v1/courses/:id",
            get(handlers::get_course)
                .patch(handlers::update_course)
                .delete(handlers::delete_course),
        )
        .route("/api/v1/courses/:id/publish", post(handlers::publish_course))
        .route("/api/v1/courses/:id/tree", get(handlers::get_course_tree))
        .route("/api/v1/courses/:id/nodes", post(handlers::create_root_node))
        // Nodes (tenant-scoped)
        .route(
            "/api/v1/nodes/:id",
            get(handlers::get_node)
                .patch(handlers::update_node)
                .delete(handlers::delete_node),
        )
        .route("/api/v1/nodes/:id/children", post(handlers::create_child_node))
        .route("/api/v1/nodes/:id/subtree", get(handlers::get_node_subtree))
        .route("/api/v1/nodes/:id/move", post(handlers::move_node))
        // JWT middleware для всех защищённых маршрутов
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(axum::Extension(jwt_manager))
}