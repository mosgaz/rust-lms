// crates/api/tests/batch_test.rs
//! Интеграционные тесты для потоков (Batches) и зачислений (Enrollments).

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

/// Тест 1: Создание потока
#[sqlx::test(migrations = "migrations")]
async fn test_create_batch(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "batch-tenant").await;
    let identity_id = common::create_test_identity(&pool, "batch@example.com", tenant_id).await;
    let _user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({
            "title": "Test Batch",
            "description": "Test description",
            "status": "draft"
        }).to_string()))
        .unwrap();

    let resp = router.oneshot(create_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(batch_json["data"]["title"], "Test Batch");
    assert_eq!(batch_json["data"]["status"], "draft");
}

/// Тест 2: Зачисление пользователя в поток
#[sqlx::test(migrations = "migrations")]
async fn test_enroll_user_to_batch(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "enroll-tenant").await;
    let identity_id = common::create_test_identity(&pool, "enroll@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём поток
    let create_batch_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Batch" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_batch_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    // Act: Зачисляем пользователя
    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/batches/{}/enroll", batch_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({
            "user_id": user_id.0,
            "role": "student"
        }).to_string()))
        .unwrap();

    let resp = router.oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let enrollment_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(enrollment_json["data"]["role"], "student");
    assert_eq!(enrollment_json["data"]["status"], "active");
}

/// Тест 3: Попытка двойного зачисления → 409 Conflict
#[sqlx::test(migrations = "migrations")]
async fn test_double_enrollment_conflict(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "conflict-tenant").await;
    let identity_id = common::create_test_identity(&pool, "conflict@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём поток
    let create_batch_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Batch" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_batch_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    // Первое зачисление
    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/batches/{}/enroll", batch_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "user_id": user_id.0 }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Act: Повторное зачисление
    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/batches/{}/enroll", batch_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "user_id": user_id.0 }).to_string()))
        .unwrap();

    let resp = router.oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// Тест 4: Отчисление из потока (soft delete)
#[sqlx::test(migrations = "migrations")]
async fn test_unenroll_from_batch(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "unenroll-tenant").await;
    let identity_id = common::create_test_identity(&pool, "unenroll@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём поток и зачисляем пользователя
    let create_batch_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Batch" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_batch_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/batches/{}/enroll", batch_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "user_id": user_id.0 }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Act: Отчисляем пользователя
    let unenroll_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/batches/{}/enroll/{}", batch_id, user_id.0))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(unenroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

/// Тест 5: Изменение роли участника потока
#[sqlx::test(migrations = "migrations")]
async fn test_update_batch_role(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "role-tenant").await;
    let identity_id = common::create_test_identity(&pool, "role@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём поток и зачисляем пользователя как student
    let create_batch_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Batch" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_batch_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/batches/{}/enroll", batch_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "user_id": user_id.0, "role": "student" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Act: Меняем роль на instructor
    let update_role_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/batches/{}/enroll/{}", batch_id, user_id.0))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "role": "instructor" }).to_string()))
        .unwrap();

    let resp = router.oneshot(update_role_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let enrollment_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(enrollment_json["data"]["role"], "instructor");
}

/// Тест 6: RLS-изоляция для потоков
#[sqlx::test(migrations = "migrations")]
async fn test_batch_rls_isolation(pool: PgPool) {
    let tenant_a = common::create_test_tenant(&pool, "rls-tenant-a").await;
    let identity_a = common::create_test_identity(&pool, "user_a@example.com", tenant_a).await;
    let _user_a = common::create_test_user(&pool, identity_a, tenant_a).await;
    let token_a = common::get_auth_token(tenant_a, identity_a);

    let tenant_b = common::create_test_tenant(&pool, "rls-tenant-b").await;
    let identity_b = common::create_test_identity(&pool, "user_b@example.com", tenant_b).await;
    let _user_b = common::create_test_user(&pool, identity_b, tenant_b).await;
    let token_b = common::get_auth_token(tenant_b, identity_b);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Tenant A создаёт поток
    let create_batch_req = Request::builder()
        .method("POST")
        .uri("/api/v1/batches")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token_a))
        .body(Body::from(json!({ "title": "Secret Batch" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_batch_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let batch_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    // Act: Tenant B пытается получить этот поток
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/batches/{}", batch_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token_b))
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(get_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Тест 7: Индивидуальное зачисление на курс
#[sqlx::test(migrations = "migrations")]
async fn test_enroll_user_to_course(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "course-enroll-tenant").await;
    let identity_id = common::create_test_identity(&pool, "course_enroll@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let router = create_router(pool.clone(), JwtConfig::default());

    // Создаём курс
    let create_course_req = Request::builder()
        .method("POST")
        .uri("/api/v1/courses")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "title": "Test Course" }).to_string()))
        .unwrap();

    let resp = router.clone().oneshot(create_course_req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let course_id = serde_json::from_slice::<Value>(&body)["data"]["id"].as_str().unwrap();

    // Act: Зачисляем пользователя на курс
    let enroll_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/courses/{}/enroll", course_id))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "user_id": user_id.0 }).to_string()))
        .unwrap();

    let resp = router.oneshot(enroll_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let enrollment_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(enrollment_json["data"]["status"], "active");
    assert_eq!(enrollment_json["data"]["progress"], 0.0);
}