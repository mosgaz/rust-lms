// crates/api/src/http/router.rs
//! Конфигурация Axum-роутера.

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;

use crate::auth::{AuthService, JwtConfig, JwtManager};
use crate::database::{
    BatchEnrollmentRepository, BatchRepository, CourseEnrollmentRepository, CourseRepository,
    IdentityRepository, LessonProgressRepository, NodeRepository, TenantRepository, UserRepository,
	QuestionRepository,
};
use crate::services::ProgressService;

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
    let batch_repo = BatchRepository::new(pool.clone());
    let batch_enrollment_repo = BatchEnrollmentRepository::new(pool.clone());
    let course_enrollment_repo = CourseEnrollmentRepository::new(pool.clone());
    let lesson_progress_repo = LessonProgressRepository::new(pool.clone());
    
    let progress_service = ProgressService::new(lesson_progress_repo);
    let jwt_manager = JwtManager::new(jwt_config);

    let auth_service = AuthService::new(
        identity_repo,
        user_repo.clone(),
        tenant_repo.clone(),
        jwt_manager.clone(),
    );

	let question_repo = QuestionRepository::new(pool.clone());

    let state = AppState {
        tenant_repo,
        user_repo,
        auth_service,
        course_repo,
        node_repo,
        batch_repo,
        batch_enrollment_repo,
        course_enrollment_repo,
        progress_service,
		question_repo,
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
        .route(
            "/api/v1/nodes/:id",
            get(handlers::get_node)
                .patch(handlers::update_node)
                .delete(handlers::delete_node),
        )
        .route("/api/v1/nodes/:id/children", post(handlers::create_child_node))
        .route("/api/v1/nodes/:id/subtree", get(handlers::get_node_subtree))
        .route("/api/v1/nodes/:id/move", post(handlers::move_node))
        .route(
            "/api/v1/batches",
            get(handlers::list_batches).post(handlers::create_batch),
        )
        .route(
            "/api/v1/batches/:id",
            get(handlers::get_batch)
                .patch(handlers::update_batch)
                .delete(handlers::delete_batch),
        )
        .route(
            "/api/v1/batches/:id/enroll",
            post(handlers::enroll_to_batch),
        )
        .route(
            "/api/v1/batches/:batch_id/enroll/:user_id",
            delete(handlers::unenroll_from_batch).patch(handlers::update_batch_enrollment_role),
        )
        .route(
            "/api/v1/batches/:id/enrollments",
            get(handlers::list_batch_enrollments),
        )
        .route(
            "/api/v1/courses/:id/enroll",
            post(handlers::enroll_to_course),
        )
        .route(
            "/api/v1/courses/:course_id/enroll/:user_id",
            delete(handlers::unenroll_from_course),
        )
        .route(
            "/api/v1/courses/:id/enrollments",
            get(handlers::list_course_enrollments),
        )
        .route(
            "/api/v1/users/:id/enrollments",
            get(handlers::list_user_course_enrollments),
        )
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(axum::Extension(jwt_manager))
}