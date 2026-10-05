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