// crates/api/tests/common/mod.rs
//! Общие вспомогательные функции для интеграционных тестов.

use rust_lms_api::{JwtConfig, JwtManager, PasswordHasher};
use rust_lms_shared::{IdentityId, TenantId, UserId};
use sqlx::PgPool;

/// Создаёт тестовый тенант.
pub async fn create_test_tenant(pool: &PgPool, slug: &str) -> TenantId {
    let tenant_id = TenantId::new();
    sqlx::query("INSERT INTO tenants (id, slug, name, is_active) VALUES ($1, $2, $3, true)")
        .bind(tenant_id.0)
        .bind(slug)
        .bind(format!("Test Tenant {}", slug))
        .execute(pool)
        .await
        .expect("Failed to create test tenant");
    tenant_id
}

/// Создаёт тестовую личность (identity).
pub async fn create_test_identity(pool: &PgPool, email: &str, tenant_id: TenantId) -> IdentityId {
    let identity_id = IdentityId::new();
    let hasher = PasswordHasher::new();
    let hash = hasher.hash("TestPassword123!").unwrap();
    
    sqlx::query(
        "INSERT INTO identities (id, email, password_hash, preferred_tenant_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(identity_id.0)
    .bind(email)
    .bind(hash)
    .bind(tenant_id.0)
    .execute(pool)
    .await
    .expect("Failed to create test identity");
    identity_id
}

/// Создаёт тестового пользователя (user).
pub async fn create_test_user(pool: &PgPool, identity_id: IdentityId, tenant_id: TenantId) -> UserId {
    let user_id = UserId::new();
    sqlx::query("INSERT INTO users (id, tenant_id, identity_id, is_active) VALUES ($1, $2, $3, true)")
        .bind(user_id.0)
        .bind(tenant_id.0)
        .bind(identity_id.0)
        .execute(pool)
        .await
        .expect("Failed to create test user");
    user_id
}

/// Генерирует JWT-токен для аутентификации.
pub fn get_auth_token(tenant_id: TenantId, identity_id: IdentityId) -> String {
    let jwt_manager = JwtManager::new(JwtConfig::default());
    jwt_manager.generate_access_token(identity_id, tenant_id).unwrap()
}

/// Создаёт тестовый курс (без иерархии уроков).
pub async fn create_test_course(pool: &PgPool, tenant_id: TenantId, title: &str) -> uuid::Uuid {
    let course_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO courses (id, tenant_id, title, description, status, metadata)
         VALUES ($1, $2, $3, '', 'draft', '{}'::jsonb)",
    )
    .bind(course_id)
    .bind(tenant_id.0)
    .bind(title)
    .execute(pool)
    .await
    .expect("Failed to create test course");
    course_id
}

/// Создаёт тестовый урок (lesson) в указанном курсе.
///
/// # Arguments
/// * `weight` - вес урока (для взвешенного расчёта прогресса), None = 1.0
/// * `quiz_passing_score` - если Some, урок считается тестом с порогом сдачи
pub async fn create_test_lesson(
    pool: &PgPool,
    tenant_id: TenantId,
    course_id: uuid::Uuid,
    title: &str,
    weight: Option<f64>,
    quiz_passing_score: Option<f64>,
) -> uuid::Uuid {
    let lesson_id = uuid::Uuid::new_v4();

    let metadata = match (weight, quiz_passing_score) {
        (Some(w), Some(ps)) => serde_json::json!({"weight": w, "quiz": {"passing_score": ps}}),
        (Some(w), None) => serde_json::json!({"weight": w}),
        (None, Some(ps)) => serde_json::json!({"quiz": {"passing_score": ps}}),
        (None, None) => serde_json::json!({}),
    };

    sqlx::query(
        "INSERT INTO nodes (id, tenant_id, course_id, node_type, title, metadata, is_archived)
         VALUES ($1, $2, $3, 'lesson', $4, $5::jsonb, false)",
    )
    .bind(lesson_id)
    .bind(tenant_id.0)
    .bind(course_id)
    .bind(title)
    .bind(metadata)
    .execute(pool)
    .await
    .expect("Failed to create test lesson");
    lesson_id
}

/// Создаёт архивный урок (is_archived = true).
pub async fn create_archived_lesson(
    pool: &PgPool,
    tenant_id: TenantId,
    course_id: uuid::Uuid,
    title: &str,
) -> uuid::Uuid {
    let lesson_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO nodes (id, tenant_id, course_id, node_type, title, metadata, is_archived)
         VALUES ($1, $2, $3, 'lesson', $4, '{}'::jsonb, true)",
    )
    .bind(lesson_id)
    .bind(tenant_id.0)
    .bind(course_id)
    .bind(title)
    .execute(pool)
    .await
    .expect("Failed to create archived lesson");
    lesson_id
}

/// Зачисляет пользователя на курс (индивидуальное зачисление).
pub async fn enroll_user_to_course(
    pool: &PgPool,
    tenant_id: TenantId,
    user_id: UserId,
    course_id: uuid::Uuid,
) {
    let enrollment_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO course_enrollments (id, tenant_id, user_id, course_id, status, progress, completed_lessons_weight)
         VALUES ($1, $2, $3, $4, 'active', 0.0, 0.0)",
    )
    .bind(enrollment_id)
    .bind(tenant_id.0)
    .bind(user_id.0)
    .bind(course_id)
    .execute(pool)
    .await
    .expect("Failed to enroll user to course");
}

/// Устанавливает критерии завершения для курса.
pub async fn set_course_completion_criteria(
    pool: &PgPool,
    course_id: uuid::Uuid,
    criteria: &serde_json::Value,
) {
    sqlx::query(
        "UPDATE courses SET metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{completion_criteria}', $2::jsonb)
         WHERE id = $1",
    )
    .bind(course_id)
    .bind(criteria)
    .execute(pool)
    .await
    .expect("Failed to set completion criteria");
}