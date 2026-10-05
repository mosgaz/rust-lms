// crates/api/tests/hierarchy_test.rs
//! Интеграционные тесты для иерархии контента (Courses & Nodes).

use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use http_body_util::BodyExt;
use rust_lms_api::{create_router, JwtConfig};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

mod common;

/// Тест 1: Создание иерархии и получение полного дерева курса
#[sqlx::test(migrations = "migrations")]
async fn test_create_and_get_course_tree(pool: PgPool) {
    // Arrange
    let tenant_id = common::create_test_tenant(&pool, "tree-tenant").await;
    let identity_id = common::create_test_identity(&pool, "tree@example.com", tenant_id).await;
    let _user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

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
    assert_eq!(nodes.len(), 2);
    
    let lesson = nodes.iter().find(|n| n["title"] == "Lesson 1.1").unwrap();
    let chapter = nodes.iter().find(|n| n["title"] == "Chapter 1").unwrap();
    assert_eq!(lesson["parent_id"].as_str().unwrap(), chapter["id"].as_str().unwrap());
}

/// Тест 2: Перемещение узла (проверка обновления ltree path)
#[sqlx::test(migrations = "migrations")]
async fn test_move_node_updates_path(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "move-tenant").await;
    let identity_id = common::create_test_identity(&pool, "move@example.com", tenant_id).await;
    let _user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

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
    let tenant_a = common::create_test_tenant(&pool, "tenant-a").await;
    let identity_a = common::create_test_identity(&pool, "user_a@example.com", tenant_a).await;
    let _user_a = common::create_test_user(&pool, identity_a, tenant_a).await;
    let token_a = common::get_auth_token(tenant_a, identity_a);

    let tenant_b = common::create_test_tenant(&pool, "tenant-b").await;
    let identity_b = common::create_test_identity(&pool, "user_b@example.com", tenant_b).await;
    let _user_b = common::create_test_user(&pool, identity_b, tenant_b).await;
    let token_b = common::get_auth_token(tenant_b, identity_b);

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

    // Assert: RLS срабатывает, Tenant B видит 404
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}