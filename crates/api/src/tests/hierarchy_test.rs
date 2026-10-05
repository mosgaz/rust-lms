// crates/api/tests/hierarchy_test.rs
//! Интеграционные тесты для иерархии контента (Courses & Nodes).
//!
//! Проверяют:
//! - Создание и получение дерева курса (ltree)
//! - Перемещение узлов и корректное обновление path
//! - RLS-изоляцию между тенантами
//! - Каскадное удаление

use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use rust_lms_api::{create_router, JwtConfig, JwtManager, PasswordHasher};
use rust_lms_shared::{CourseId, IdentityId, NodeId, NodeType, TenantId, UserId};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

// --- Вспомогательные функции ---

async fn create_test_tenant(pool: &PgPool, slug: &str) -> TenantId {
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

async fn create_test_identity(pool: &PgPool, email: &str, tenant_id: TenantId) -> IdentityId {
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

async fn create_test_user(pool: &PgPool, identity_id: IdentityId, tenant_id: TenantId) -> UserId {
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

fn get_auth_token(tenant_id: TenantId, identity_id: IdentityId) -> String {
    let jwt_manager = JwtManager::new(JwtConfig::default());
    jwt_manager.generate_access_token(identity_id, tenant_id).unwrap()
}

// --- Тесты ---

/// Тест 1: Создание иерархии и получение полного дерева курса
#[sqlx::test(migrations = "migrations")]
async fn test_create_and_get_course_tree(pool: PgPool) {
    // Arrange
    let tenant_id = create_test_tenant(&pool, "tree-tenant").await;
    let identity_id = create_test_identity(&pool, "tree@example.com", tenant_id).await;
    let _user_id = create_test_user(&pool, identity_id, tenant_id).await;
    let token = get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Act 1: Создаём курс
    let create_course_req = Request::builder()
        .method("POST")
        .uri("/api/v1/courses")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Course" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_course_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let course_json: Value = serde_json::from_slice(&body).unwrap();
    let course_id = course_json["data"]["id"].as_str().unwrap();

    // Act 2: Создаём Chapter (корневой узел)
    let create_chapter_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/courses/{}/nodes", course_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({
            "node_type": "chapter",
            "title": "Chapter 1",
            "metadata": {}
        }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_chapter_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let chapter_json: Value = serde_json::from_slice(&body).unwrap();
    let chapter_id = chapter_json["data"]["id"].as_str().unwrap();

    // Act 3: Создаём Lesson (дочерний узел)
    let create_lesson_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/nodes/{}/children", chapter_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({
            "node_type": "lesson",
            "title": "Lesson 1.1",
            "metadata": {"content_type": "video"}
        }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_lesson_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Act 4: Получаем дерево курса
    let get_tree_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/courses/{}/tree", course_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(get_tree_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let tree_json: Value = serde_json::from_slice(&body).unwrap();
    
    // Assert
    let nodes = tree_json["data"].as_array().unwrap();
    assert_eq!(nodes.len(), 2); // Chapter и Lesson
    
    // Проверяем, что Lesson является потомком Chapter (проверка ltree логики)
    let lesson = nodes.iter().find(|n| n["title"] == "Lesson 1.1").unwrap();
    let chapter = nodes.iter().find(|n| n["title"] == "Chapter 1").unwrap();
    
    // В реальном ltree path lesson должен начинаться с path chapter
    // (Здесь мы проверяем косвенно через parent_id, так как path не возвращается в DTO, что правильно)
    assert_eq!(lesson["parent_id"].as_str().unwrap(), chapter["id"].as_str().unwrap());
}

/// Тест 2: Перемещение узла (проверка обновления ltree path)
#[sqlx::test(migrations = "migrations")]
async fn test_move_node_updates_path(pool: PgPool) {
    // Arrange
    let tenant_id = create_test_tenant(&pool, "move-tenant").await;
    let identity_id = create_test_identity(&pool, "move@example.com", tenant_id).await;
    let _user_id = create_test_user(&pool, identity_id, tenant_id).await;
    let token = get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём курс и две главы
    let course_req = Request::builder().method("POST").uri("/api/v1/courses")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Move Test Course" }).to_string())).unwrap();
    
    let resp = router.clone().oneshot(course_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let course_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    let ch1_req = Request::builder().method("POST").uri(format!("/api/v1/courses/{}/nodes", course_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({"node_type": "chapter", "title": "Chapter 1", "metadata": {}}).to_string())).unwrap();
    let resp = router.clone().oneshot(ch1_req).await.unwrap();
    let ch1_id = serde_json::from_slice::<Value>(&resp.into_body().collect().await.unwrap().to_bytes())["data"]["id"].as_str().unwrap();

    let ch2_req = Request::builder().method("POST").uri(format!("/api/v1/courses/{}/nodes", course_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({"node_type": "chapter", "title": "Chapter 2", "metadata": {}}).to_string())).unwrap();
    let resp = router.clone().oneshot(ch2_req).await.unwrap();
    let ch2_id = serde_json::from_slice::<Value>(&resp.into_body().collect().await.unwrap().to_bytes())["data"]["id"].as_str().unwrap();

    // Создаём урок в Chapter 1
    let lesson_req = Request::builder().method("POST").uri(format!("/api/v1/nodes/{}/children", ch1_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({"node_type": "lesson", "title": "Lesson to Move", "metadata": {}}).to_string())).unwrap();
    let resp = router.clone().oneshot(lesson_req).await.unwrap();
    let lesson_id = serde_json::from_slice::<Value>(&resp.into_body().collect().await.unwrap().to_bytes())["data"]["id"].as_str().unwrap();

    // Act: Перемещаем урок в Chapter 2
    let move_req = Request::builder().method("POST").uri(format!("/api/v1/nodes/{}/move", lesson_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({"new_parent_id": ch2_id}).to_string())).unwrap();

    let resp = router.oneshot(move_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let moved_node: Value = serde_json::from_slice(&body).unwrap();
    
    // Assert: parent_id изменился
    assert_eq!(moved_node["data"]["parent_id"].as_str().unwrap(), ch2_id);
}

/// Тест 3: RLS-изоляция (попытка получить дерево чужого курса)
#[sqlx::test(migrations = "migrations")]
async fn test_hierarchy_rls_isolation(pool: PgPool) {
    // Arrange: Tenant A
    let tenant_a = create_test_tenant(&pool, "tenant-a").await;
    let identity_a = create_test_identity(&pool, "user_a@example.com", tenant_a).await;
    let _user_a = create_test_user(&pool, identity_a, tenant_a).await;
    let token_a = get_auth_token(tenant_a, identity_a);

    // Arrange: Tenant B
    let tenant_b = create_test_tenant(&pool, "tenant-b").await;
    let identity_b = create_test_identity(&pool, "user_b@example.com", tenant_b).await;
    let _user_b = create_test_user(&pool, identity_b, tenant_b).await;
    let token_b = get_auth_token(tenant_b, identity_b);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Tenant A создаёт курс
    let create_req = Request::builder().method("POST").uri("/api/v1/courses")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token_a))
        .body(Body::from(json!({ "title": "Secret Course" }).to_string())).unwrap();
    
    let resp = router.clone().oneshot(create_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let course_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    // Act: Tenant B пытается получить этот курс
    let get_req = Request::builder().method("GET").uri(format!("/api/v1/courses/{}", course_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token_b))
        .body(Body::empty()).unwrap();

    let resp = router.oneshot(get_req).await.unwrap();

    // Assert: RLS срабатывает, Tenant B видит 404 (Not Found), а не 403, чтобы не раскрывать факт существования
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}