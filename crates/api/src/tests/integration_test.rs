// crates/api/tests/integration_test.rs
//! Интеграционные тесты для HTTP-слоя с Identity-First архитектурой.
//!
//! Эти тесты проверяют полный цикл аутентификации, включая:
//! - Двухшаговый поток логина (login → select_tenant)
//! - Авто-выбор тенанта через preferred_tenant_id
//! - RLS-изоляцию между тенантами
//! - Защиту от подмены tenant_id

use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use rust_lms_api::{create_router, JwtConfig, PasswordHasher};
use rust_lms_shared::{IdentityId, TenantId, UserId};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

/// Вспомогательная функция для создания тестового тенанта
async fn create_test_tenant(pool: &PgPool, slug: &str) -> TenantId {
    let tenant_id = TenantId::new();
    sqlx::query(
        "INSERT INTO tenants (id, slug, name, is_active) VALUES ($1, $2, $3, true)"
    )
    .bind(tenant_id.0)
    .bind(slug)
    .bind(format!("Test Tenant {}", slug))
    .execute(pool)
    .await
    .expect("Failed to create test tenant");
    tenant_id
}

/// Вспомогательная функция для создания тестовой identity
async fn create_test_identity(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    preferred_tenant_id: Option<TenantId>,
) -> IdentityId {
    let identity_id = IdentityId::new();
    sqlx::query(
        "INSERT INTO identities (id, email, password_hash, preferred_tenant_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(identity_id.0)
    .bind(email)
    .bind(password_hash)
    .bind(preferred_tenant_id.map(|t| t.0))
    .execute(pool)
    .await
    .expect("Failed to create test identity");
    identity_id
}

/// Вспомогательная функция для создания связи user-tenant
async fn create_test_user(pool: &PgPool, identity_id: IdentityId, tenant_id: TenantId) -> UserId {
    let user_id = UserId::new();
    sqlx::query(
        "INSERT INTO users (id, tenant_id, identity_id, is_active) VALUES ($1, $2, $3, true)"
    )
    .bind(user_id.0)
    .bind(tenant_id.0)
    .bind(identity_id.0)
    .execute(pool)
    .await
    .expect("Failed to create test user");
    user_id
}

/// Тест 1: Успешный логин с авто-выбором (preferred_tenant_id установлен и активен)
#[sqlx::test(migrations = "migrations")]
async fn test_login_with_auto_select(pool: PgPool) {
    // Arrange
    let tenant_id = create_test_tenant(&pool, "auto-tenant").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "auto@example.com", &password_hash, Some(tenant_id)).await;
    let _user_id = create_test_user(&pool, identity_id, tenant_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool, jwt_config);

    // Act: Login
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "auto@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();

    // Assert: Должны получить финальные токены (SingleTenant)
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["access_token"].is_string());
    assert!(json["data"]["refresh_token"].is_string());
    assert_eq!(json["data"]["token_type"].as_str().unwrap(), "Bearer");
    // При авто-выборе session_token не возвращается
    assert!(json["data"]["session_token"].is_null());
}

/// Тест 2: Логин с ручным выбором (preferred_tenant_id отсутствует)
#[sqlx::test(migrations = "migrations")]
async fn test_login_with_manual_select(pool: PgPool) {
    // Arrange
    let tenant1_id = create_test_tenant(&pool, "manual-tenant-1").await;
    let tenant2_id = create_test_tenant(&pool, "manual-tenant-2").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "manual@example.com", &password_hash, None).await;
    let _user1_id = create_test_user(&pool, identity_id, tenant1_id).await;
    let _user2_id = create_test_user(&pool, identity_id, tenant2_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool.clone(), jwt_config);

    // Act 1: Login (должен вернуть список тенантов)
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "manual@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();

    // Assert 1: Получаем session_token и список тенантов (MultiTenant)
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["session_token"].is_string());
    assert!(json["data"]["available_tenants"].is_array());
    let tenants = json["data"]["available_tenants"].as_array().unwrap();
    assert_eq!(tenants.len(), 2);
    // При ручном выборе финальные токены не возвращаются
    assert!(json["data"]["access_token"].is_null());

    let session_token = json["data"]["session_token"].as_str().unwrap();

    // Act 2: Select tenant
    let router2 = create_router(pool, JwtConfig::default());
    let select_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/select-tenant")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "session_token": session_token,
                "tenant_id": tenant1_id.0.to_string()
            })
            .to_string(),
        ))
        .unwrap();

    let response2 = router2.oneshot(select_request).await.unwrap();

    // Assert 2: Получаем финальные токены
    assert_eq!(response2.status(), StatusCode::OK);
    let body2 = response2.into_body().collect().await.unwrap().to_bytes();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    
    assert!(json2["success"].as_bool().unwrap());
    assert!(json2["data"]["access_token"].is_string());
    assert!(json2["data"]["refresh_token"].is_string());
}

/// Тест 3: Проверка обновления preferred_tenant_id после select_tenant
#[sqlx::test(migrations = "migrations")]
async fn test_select_tenant_updates_preferred(pool: PgPool) {
    // Arrange
    let tenant1_id = create_test_tenant(&pool, "preferred-tenant-1").await;
    let tenant2_id = create_test_tenant(&pool, "preferred-tenant-2").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "preferred@example.com", &password_hash, None).await;
    let _user1_id = create_test_user(&pool, identity_id, tenant1_id).await;
    let _user2_id = create_test_user(&pool, identity_id, tenant2_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool.clone(), jwt_config);

    // Act 1: Login
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "preferred@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let session_token = json["data"]["session_token"].as_str().unwrap();

    // Act 2: Select tenant2
    let router2 = create_router(pool.clone(), JwtConfig::default());
    let select_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/select-tenant")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "session_token": session_token,
                "tenant_id": tenant2_id.0.to_string()
            })
            .to_string(),
        ))
        .unwrap();

    let response2 = router2.oneshot(select_request).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Assert: Проверяем, что preferred_tenant_id обновился
    let preferred: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT preferred_tenant_id FROM identities WHERE email = $1"
    )
    .bind("preferred@example.com")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(preferred, Some(tenant2_id.0));
}

/// Тест 4: Защита от подмены tenant_id (попытка выбрать чужой тенант)
#[sqlx::test(migrations = "migrations")]
async fn test_select_tenant_with_invalid_tenant_id(pool: PgPool) {
    // Arrange
    let tenant1_id = create_test_tenant(&pool, "valid-tenant").await;
    let tenant2_id = create_test_tenant(&pool, "invalid-tenant").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "invalid@example.com", &password_hash, None).await;
    // Создаём связь только с tenant1
    let _user1_id = create_test_user(&pool, identity_id, tenant1_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool, jwt_config);

    // Act 1: Login
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "invalid@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let session_token = json["data"]["session_token"].as_str().unwrap();

    // Act 2: Попытка выбрать tenant2 (к которому нет доступа)
    let router2 = create_router(pool, JwtConfig::default());
    let select_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/select-tenant")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "session_token": session_token,
                "tenant_id": tenant2_id.0.to_string()
            })
            .to_string(),
        ))
        .unwrap();

    let response2 = router2.oneshot(select_request).await.unwrap();

    // Assert: Должны получить ошибку доступа (403 или 401)
    assert!(
        response2.status() == StatusCode::UNAUTHORIZED 
        || response2.status() == StatusCode::FORBIDDEN
    );
}

/// Тест 5: RLS-изоляция (попытка получить пользователя из другого тенанта)
#[sqlx::test(migrations = "migrations")]
async fn test_rls_isolation(pool: PgPool) {
    // Arrange: Создаём два тенанта с разными пользователями
    let tenant1_id = create_test_tenant(&pool, "rls-tenant-1").await;
    let tenant2_id = create_test_tenant(&pool, "rls-tenant-2").await;
    
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    
    // Identity 1 в tenant1
    let identity1_id = create_test_identity(&pool, "user1@example.com", &password_hash, Some(tenant1_id)).await;
    let user1_id = create_test_user(&pool, identity1_id, tenant1_id).await;
    
    // Identity 2 в tenant2
    let identity2_id = create_test_identity(&pool, "user2@example.com", &password_hash, Some(tenant2_id)).await;
    let user2_id = create_test_user(&pool, identity2_id, tenant2_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool, jwt_config);

    // Act 1: Логинимся как user1 (получаем токен для tenant1)
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "user1@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["data"]["access_token"].as_str().unwrap();

    // Act 2: Пытаемся получить user2 из tenant2, используя токен tenant1
    let router2 = create_router(pool, JwtConfig::default());
    let get_user_request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/users/{}", user2_id.0))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response2 = router2.oneshot(get_user_request).await.unwrap();

    // Assert: RLS должен отфильтровать запрос, возвращаем 404 (пользователь не найден в контексте tenant1)
    assert_eq!(response2.status(), StatusCode::NOT_FOUND);
}

/// Тест 6: Неверные учётные данные
#[sqlx::test(migrations = "migrations")]
async fn test_login_with_invalid_credentials(pool: PgPool) {
    // Arrange
    let tenant_id = create_test_tenant(&pool, "invalid-creds-tenant").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("CorrectPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "creds@example.com", &password_hash, Some(tenant_id)).await;
    let _user_id = create_test_user(&pool, identity_id, tenant_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool, jwt_config);

    // Act: Login с неверным паролем
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "creds@example.com",
                "password": "WrongPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();

    // Assert: Должны получить 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Тест 7: Refresh token
#[sqlx::test(migrations = "migrations")]
async fn test_refresh_token(pool: PgPool) {
    // Arrange
    let tenant_id = create_test_tenant(&pool, "refresh-tenant").await;
    let hasher = PasswordHasher::new();
    let password_hash = hasher.hash("TestPassword123!").unwrap();
    let identity_id = create_test_identity(&pool, "refresh@example.com", &password_hash, Some(tenant_id)).await;
    let _user_id = create_test_user(&pool, identity_id, tenant_id).await;

    let jwt_config = JwtConfig::default();
    let router = create_router(pool.clone(), jwt_config);

    // Act 1: Login
    let login_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "refresh@example.com",
                "password": "TestPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(login_request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let refresh_token = json["data"]["refresh_token"].as_str().unwrap();

    // Act 2: Refresh
    let router2 = create_router(pool, JwtConfig::default());
    let refresh_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": refresh_token
            })
            .to_string(),
        ))
        .unwrap();

    let response2 = router2.oneshot(refresh_request).await.unwrap();

    // Assert: Должны получить новые токены
    assert_eq!(response2.status(), StatusCode::OK);
    let body2 = response2.into_body().collect().await.unwrap().to_bytes();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    
    assert!(json2["success"].as_bool().unwrap());
    assert!(json2["data"]["access_token"].is_string());
    assert!(json2["data"]["refresh_token"].is_string());
}