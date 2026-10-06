// crates/api/tests/lesson_progress_test.rs
//! Интеграционные тесты для отслеживания прогресса обучения (Этап 9).
//!
//! Покрытие: 18 сценариев
//! - Базовые операции (создание, обновление, идемпотентность)
//! - Пересчёт прогресса (равные/взвешенные веса)
//! - Критерии завершения (4 правила + AllOf/AnyOf)
//! - Защита от ошибок (404, 403, 409, 410)
//! - Частичное обновление и поведение `passed`
//! - RLS-изоляция между тенантами

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

// ============================================================================
// Хелпер: создание полного окружения (tenant + user + course + 3 урока)
// ============================================================================

struct TestEnv {
    tenant_id: rust_lms_shared::TenantId,
    user_id: rust_lms_shared::UserId,
    token: String,
    course_id: uuid::Uuid,
    lesson_ids: Vec<uuid::Uuid>,
}

async fn setup_test_env(
    pool: &PgPool,
    tenant_slug: &str,
    lesson_count: usize,
) -> TestEnv {
    let tenant_id = common::create_test_tenant(pool, tenant_slug).await;
    let identity_id = common::create_test_identity(pool, &format!("{tenant_slug}@example.com"), tenant_id).await;
    let user_id = common::create_test_user(pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(pool, tenant_id, &format!("Course for {tenant_slug}")).await;
    common::enroll_user_to_course(pool, tenant_id, user_id, course_id).await;

    let mut lesson_ids = Vec::with_capacity(lesson_count);
    for i in 0..lesson_count {
        let lesson_id = common::create_test_lesson(
            pool,
            tenant_id,
            course_id,
            &format!("Lesson {}", i + 1),
            None,
            None,
        )
        .await;
        lesson_ids.push(lesson_id);
    }

    TestEnv {
        tenant_id,
        user_id,
        token,
        course_id,
        lesson_ids,
    }
}

/// Хелпер: отправляет POST /api/v1/progress
async fn post_progress(
    pool: PgPool,
    token: &str,
    node_id: uuid::Uuid,
    payload: Value,
) -> axum::response::Response {
    let router = create_router(pool, JwtConfig::default());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/progress")
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(payload.to_string()))
        .unwrap();
    router.oneshot(req).await.unwrap()
}

/// Хелпер: парсит JSON из ответа и извлекает `data`
async fn extract_data(resp: axum::response::Response) -> Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["data"].clone()
}

// ============================================================================
// Группа 1: Базовые операции
// ============================================================================

/// Тест 1: Создание прогресса урока (upsert впервые)
#[sqlx::test(migrations = "migrations")]
async fn test_create_lesson_progress(pool: PgPool) {
    let env = setup_test_env(&pool, "create", 3).await;
    let node_id = env.lesson_ids[0];

    let resp = post_progress(
        pool,
        &env.token,
        node_id,
        json!({"node_id": node_id, "status": "in_progress", "time_spent_seconds": 120}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    assert_eq!(data["lesson_progress"]["status"], "in_progress");
    assert_eq!(data["lesson_progress"]["time_spent_seconds"], 120);
}

/// Тест 2: Обновление прогресса урока (upsert повторно)
#[sqlx::test(migrations = "migrations")]
async fn test_update_lesson_progress(pool: PgPool) {
    let env = setup_test_env(&pool, "update", 3).await;
    let node_id = env.lesson_ids[0];

    // Первое обновление
    let resp1 = post_progress(
        pool.clone(),
        &env.token,
        node_id,
        json!({"node_id": node_id, "status": "in_progress"}),
    )
    .await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Второе обновление — переход в completed
    let resp2 = post_progress(
        pool,
        &env.token,
        node_id,
        json!({"node_id": node_id, "status": "completed"}),
    )
    .await;

    assert_eq!(resp2.status(), StatusCode::OK);
    let data = extract_data(resp2).await;
    assert_eq!(data["lesson_progress"]["status"], "completed");
    assert!(data["lesson_progress"]["completed_at"].as_str().is_some());
}

/// Тест 3: Идемпотентность (повторный запрос с теми же данными)
#[sqlx::test(migrations = "migrations")]
async fn test_idempotent_progress_update(pool: PgPool) {
    let env = setup_test_env(&pool, "idempotent", 3).await;
    let node_id = env.lesson_ids[0];

    let payload = json!({"node_id": node_id, "status": "completed"});

    let resp1 = post_progress(pool.clone(), &env.token, node_id, payload.clone()).await;
    let data1 = extract_data(resp1).await;
    let completed_at_1 = data1["lesson_progress"]["completed_at"].as_str().unwrap();

    // Небольшая пауза, чтобы убедиться, что время изменилось бы при повторной записи
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let resp2 = post_progress(pool, &env.token, node_id, payload).await;
    let data2 = extract_data(resp2).await;
    let completed_at_2 = data2["lesson_progress"]["completed_at"].as_str().unwrap();

    // completed_at не должен измениться при повторном запросе
    assert_eq!(completed_at_1, completed_at_2);
}

// ============================================================================
// Группа 2: Пересчёт прогресса курса
// ============================================================================

/// Тест 4: Пересчёт прогресса курса (равные веса)
/// 3 урока, завершен 1 → progress ≈ 0.333
#[sqlx::test(migrations = "migrations")]
async fn test_course_progress_equal_weights(pool: PgPool) {
    let env = setup_test_env(&pool, "equal-weights", 3).await;
    let node_id = env.lesson_ids[0];

    let resp = post_progress(
        pool,
        &env.token,
        node_id,
        json!({"node_id": node_id, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    let progress = data["course_progress"].as_f64().unwrap();

    // 1 из 3 уроков завершён, равные веса → ≈0.333
    assert!((progress - 1.0 / 3.0).abs() < 0.001, "progress was {progress}");
    assert_eq!(data["course_status"], "active");
}

/// Тест 5: Пересчёт прогресса курса (взвешенные уроки)
/// 3 урока: веса 1.0, 2.0, 1.0 → total_weight = 4.0
/// Завершён урок с весом 2.0 → progress = 0.5
#[sqlx::test(migrations = "migrations")]
async fn test_course_progress_weighted(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "weighted").await;
    let identity_id = common::create_test_identity(&pool, "weighted@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Weighted Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    let lesson_light = common::create_test_lesson(&pool, tenant_id, course_id, "Light", Some(1.0), None).await;
    let lesson_heavy = common::create_test_lesson(&pool, tenant_id, course_id, "Heavy", Some(2.0), None).await;
    let _lesson_other = common::create_test_lesson(&pool, tenant_id, course_id, "Other", Some(1.0), None).await;

    // Завершаем только "тяжёлый" урок (вес 2.0 из 4.0)
    let resp = post_progress(
        pool,
        &token,
        lesson_heavy,
        json!({"node_id": lesson_heavy, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    let progress = data["course_progress"].as_f64().unwrap();

    // 2.0 / 4.0 = 0.5
    assert!((progress - 0.5).abs() < 0.001, "progress was {progress}");

    // Sanity: завершим лёгкий урок → должно стать 3.0/4.0 = 0.75
    let resp2 = post_progress(
        pool,
        &token,
        lesson_light,
        json!({"node_id": lesson_light, "status": "completed"}),
    )
    .await;
    let data2 = extract_data(resp2).await;
    let progress2 = data2["course_progress"].as_f64().unwrap();
    assert!((progress2 - 0.75).abs() < 0.001, "progress was {progress2}");
}

// ============================================================================
// Группа 3: Критерии завершения курса
// ============================================================================

/// Тест 6: Автозавершение по правилу `min_progress` (AllOf)
#[sqlx::test(migrations = "migrations")]
async fn test_auto_complete_by_min_progress(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "min-prog").await;
    let identity_id = common::create_test_identity(&pool, "min-prog@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Min Progress Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    // Устанавливаем критерий: min_progress = 0.5
    common::set_course_completion_criteria(
        &pool,
        course_id,
        &json!({"mode": "all_of", "rules": [{"type": "min_progress", "value": 0.5}]}),
    )
    .await;

    let lesson1 = common::create_test_lesson(&pool, tenant_id, course_id, "L1", None, None).await;
    let lesson2 = common::create_test_lesson(&pool, tenant_id, course_id, "L2", None, None).await;

    // Завершаем первый урок → progress = 0.5, должен сработать критерий
    let resp = post_progress(
        pool,
        &token,
        lesson1,
        json!({"node_id": lesson1, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    assert_eq!(data["course_status"], "completed");
    assert!(data["completion_triggered"].as_bool().unwrap());

    // Sanity: второй урок уже не должен обновлять прогресс (409 Conflict)
    let resp2 = post_progress(
        pool,
        &token,
        lesson2,
        json!({"node_id": lesson2, "status": "completed"}),
    )
    .await;
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}

/// Тест 7: Автозавершение по правилу `required_nodes`
#[sqlx::test(migrations = "migrations")]
async fn test_auto_complete_by_required_nodes(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "req-nodes").await;
    let identity_id = common::create_test_identity(&pool, "req-nodes@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Required Nodes Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    let optional = common::create_test_lesson(&pool, tenant_id, course_id, "Optional", None, None).await;
    let required = common::create_test_lesson(&pool, tenant_id, course_id, "Required", None, None).await;

    // Устанавливаем критерий: required_nodes = [required]
    common::set_course_completion_criteria(
        &pool,
        course_id,
        &json!({"mode": "all_of", "rules": [{"type": "required_nodes", "node_ids": [required.to_string()]}]}),
    )
    .await;

    // Завершаем только обязательный урок → курс должен завершиться
    let resp = post_progress(
        pool,
        &token,
        required,
        json!({"node_id": required, "status": "completed"}),
    )
    .await;

    let data = extract_data(resp).await;
    assert_eq!(data["course_status"], "completed");
    assert!(data["completion_triggered"].as_bool().unwrap());

    // Проверка: опциональный урок так и остался незавершённым
    // (мы это видим косвенно через course_progress < 1.0)
    let progress = data["course_progress"].as_f64().unwrap();
    assert!(progress < 1.0, "progress was {progress}");
}

/// Тест 8: Автозавершение по правилу `all_lessons_completed`
#[sqlx::test(migrations = "migrations")]
async fn test_auto_complete_by_all_lessons(pool: PgPool) {
    let env = setup_test_env(&pool, "all-lessons", 2).await;

    // Устанавливаем критерий: все уроки должны быть завершены
    common::set_course_completion_criteria(
        &pool,
        env.course_id,
        &json!({"mode": "all_of", "rules": [{"type": "all_lessons_completed"}]}),
    )
    .await;

    // Завершаем первый урок — курс НЕ должен завершиться
    let resp1 = post_progress(
        pool.clone(),
        &env.token,
        env.lesson_ids[0],
        json!({"node_id": env.lesson_ids[0], "status": "completed"}),
    )
    .await;
    let data1 = extract_data(resp1).await;
    assert_eq!(data1["course_status"], "active");
    assert!(!data1["completion_triggered"].as_bool().unwrap());

    // Завершаем второй урок — курс должен завершиться
    let resp2 = post_progress(
        pool,
        &env.token,
        env.lesson_ids[1],
        json!({"node_id": env.lesson_ids[1], "status": "completed"}),
    )
    .await;
    let data2 = extract_data(resp2).await;
    assert_eq!(data2["course_status"], "completed");
    assert!(data2["completion_triggered"].as_bool().unwrap());
}

/// Тест 9: Режим AnyOf — одно правило выполнено → курс завершён
#[sqlx::test(migrations = "migrations")]
async fn test_auto_complete_any_of_mode(pool: PgPool) {
    let env = setup_test_env(&pool, "any-of", 3).await;

    // min_progress = 0.9 (невыполнимо на 1 уроке) ИЛИ required_nodes = [lesson_ids[0]]
    common::set_course_completion_criteria(
        &pool,
        env.course_id,
        &json!({
            "mode": "any_of",
            "rules": [
                {"type": "min_progress", "value": 0.9},
                {"type": "required_nodes", "node_ids": [env.lesson_ids[0].to_string()]}
            ]
        }),
    )
    .await;

    // Завершаем только lesson_ids[0] → второе правило сработает
    let resp = post_progress(
        pool,
        &env.token,
        env.lesson_ids[0],
        json!({"node_id": env.lesson_ids[0], "status": "completed"}),
    )
    .await;

    let data = extract_data(resp).await;
    assert_eq!(data["course_status"], "completed");
}

/// Тест 10: Режим AllOf — хотя бы одно правило НЕ выполнено → курс НЕ завершён
#[sqlx::test(migrations = "migrations")]
async fn test_all_of_requires_all_rules(pool: PgPool) {
    let env = setup_test_env(&pool, "all-of-strict", 3).await;

    // min_progress = 0.5 (выполнимо) И min_avg_quiz_score = 0.9 (невыполнимо без тестов)
    common::set_course_completion_criteria(
        &pool,
        env.course_id,
        &json!({
            "mode": "all_of",
            "rules": [
                {"type": "min_progress", "value": 0.5},
                {"type": "min_avg_quiz_score", "value": 0.9}
            ]
        }),
    )
    .await;

    // Завершаем один урок из трёх → progress = 0.33 (первое правило не выполнено тоже)
    // Даже если бы выполнилось, второе правило не выполнено → курс не завершён
    let resp = post_progress(
        pool,
        &env.token,
        env.lesson_ids[0],
        json!({"node_id": env.lesson_ids[0], "status": "completed"}),
    )
    .await;

    let data = extract_data(resp).await;
    assert_eq!(data["course_status"], "active");
    assert!(!data["completion_triggered"].as_bool().unwrap());
}

// ============================================================================
// Группа 4: Защита и ошибки
// ============================================================================

/// Тест 11: 404 — урок не найден
#[sqlx::test(migrations = "migrations")]
async fn test_progress_update_nonexistent_node(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "not-found").await;
    let identity_id = common::create_test_identity(&pool, "not-found@example.com", tenant_id).await;
    let _user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let fake_node_id = uuid::Uuid::new_v4();

    let resp = post_progress(
        pool,
        &token,
        fake_node_id,
        json!({"node_id": fake_node_id, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Тест 12: 403 — пользователь не зачислен в курс (NotEnrolled)
#[sqlx::test(migrations = "migrations")]
async fn test_progress_update_not_enrolled(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "not-enrolled").await;
    let identity_id = common::create_test_identity(&pool, "not-enrolled@example.com", tenant_id).await;
    let _user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    // Создаём курс и урок, но НЕ зачисляем пользователя
    let course_id = common::create_test_course(&pool, tenant_id, "No Enrollment Course").await;
    let lesson_id = common::create_test_lesson(&pool, tenant_id, course_id, "Lesson", None, None).await;

    let resp = post_progress(
        pool,
        &token,
        lesson_id,
        json!({"node_id": lesson_id, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// Тест 13: 409 — курс уже завершён (CourseAlreadyCompleted)
#[sqlx::test(migrations = "migrations")]
async fn test_progress_update_after_completion(pool: PgPool) {
    let env = setup_test_env(&pool, "completed-course", 1).await;

    // Завершаем единственный урок → курс автоматически завершается
    let resp1 = post_progress(
        pool.clone(),
        &env.token,
        env.lesson_ids[0],
        json!({"node_id": env.lesson_ids[0], "status": "completed"}),
    )
    .await;
    assert_eq!(resp1.status(), StatusCode::OK);
    let data1 = extract_data(resp1).await;
    assert_eq!(data1["course_status"], "completed");

    // Создаём ещё один урок и пытаемся обновить его прогресс → 409
    let new_lesson = common::create_test_lesson(&pool, env.tenant_id, env.course_id, "New Lesson", None, None).await;
    let resp2 = post_progress(
        pool,
        &env.token,
        new_lesson,
        json!({"node_id": new_lesson, "status": "completed"}),
    )
    .await;

    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}

/// Тест 14: 410 Gone — урок архивирован (NodeArchived)
#[sqlx::test(migrations = "migrations")]
async fn test_progress_update_archived_lesson(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "archived").await;
    let identity_id = common::create_test_identity(&pool, "archived@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Archived Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    let archived_lesson = common::create_archived_lesson(&pool, tenant_id, course_id, "Archived Lesson").await;

    let resp = post_progress(
        pool,
        &token,
        archived_lesson,
        json!({"node_id": archived_lesson, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::GONE);
}

// ============================================================================
// Группа 5: Частичное обновление и поведение `passed`
// ============================================================================

/// Тест 15: Частичное обновление (PATCH-семантика)
/// Обновляем только `time_spent_seconds`, не трогая `status`
#[sqlx::test(migrations = "migrations")]
async fn test_partial_progress_update(pool: PgPool) {
    let env = setup_test_env(&pool, "partial", 3).await;
    let node_id = env.lesson_ids[0];

    // Первый запрос: задаём status
    let resp1 = post_progress(
        pool.clone(),
        &env.token,
        node_id,
        json!({"node_id": node_id, "status": "in_progress", "time_spent_seconds": 60}),
    )
    .await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Второй запрос: обновляем только time_spent_seconds
    let resp2 = post_progress(
        pool,
        &env.token,
        node_id,
        json!({"node_id": node_id, "time_spent_seconds": 300}),
    )
    .await;
    assert_eq!(resp2.status(), StatusCode::OK);

    let data = extract_data(resp2).await;
    assert_eq!(data["lesson_progress"]["status"], "in_progress"); // не изменился
    assert_eq!(data["lesson_progress"]["time_spent_seconds"], 300); // обновился
}

/// Тест 16: Клиентский `passed` игнорируется, сервер вычисляет сам
/// Клиент присылает score=0.6 и passed=false, но passing_score=0.5 → passed=true
#[sqlx::test(migrations = "migrations")]
async fn test_server_computes_passed(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "passed-calc").await;
    let identity_id = common::create_test_identity(&pool, "passed@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Quiz Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    // Урок-тест с passing_score = 0.5
    let quiz_lesson = common::create_test_lesson(
        &pool,
        tenant_id,
        course_id,
        "Quiz Lesson",
        None,
        Some(0.5),
    )
    .await;

    // Клиент шлёт score=0.6 и ЛОЖНЫЙ passed=false
    let resp = post_progress(
        pool,
        &token,
        quiz_lesson,
        json!({
            "node_id": quiz_lesson,
            "status": "completed",
            "score": 0.6,
            "passed": false
        }),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    // Сервер должен вычислить passed=true, игнорируя клиентское значение
    assert_eq!(data["lesson_progress"]["score"], 0.6);
    assert_eq!(data["lesson_progress"]["passed"], true);
}

/// Тест 17: Архивный урок не учитывается в пересчёте прогресса
#[sqlx::test(migrations = "migrations")]
async fn test_archived_lesson_excluded_from_progress(pool: PgPool) {
    let tenant_id = common::create_test_tenant(&pool, "arch-exclude").await;
    let identity_id = common::create_test_identity(&pool, "arch-exclude@example.com", tenant_id).await;
    let user_id = common::create_test_user(&pool, identity_id, tenant_id).await;
    let token = common::get_auth_token(tenant_id, identity_id);

    let course_id = common::create_test_course(&pool, tenant_id, "Arch Exclude Course").await;
    common::enroll_user_to_course(&pool, tenant_id, user_id, course_id).await;

    // 2 активных урока + 1 архивный
    let lesson1 = common::create_test_lesson(&pool, tenant_id, course_id, "Active 1", None, None).await;
    let lesson2 = common::create_test_lesson(&pool, tenant_id, course_id, "Active 2", None, None).await;
    let _archived = common::create_archived_lesson(&pool, tenant_id, course_id, "Archived").await;

    // Завершаем 1 активный урок из 2 → progress = 0.5 (а не 1/3)
    let resp = post_progress(
        pool,
        &token,
        lesson1,
        json!({"node_id": lesson1, "status": "completed"}),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let data = extract_data(resp).await;
    let progress = data["course_progress"].as_f64().unwrap();
    assert!((progress - 0.5).abs() < 0.001, "progress was {progress}");
}

// ============================================================================
// Группа 6: RLS-изоляция
// ============================================================================

/// Тест 18: RLS-изоляция между тенантами
/// Студент Tenant A создаёт прогресс, студент Tenant B пытается получить — видит 404
#[sqlx::test(migrations = "migrations")]
async fn test_progress_rls_isolation(pool: PgPool) {
    // Создаём два изолированных тенанта с полным окружением
    let env_a = setup_test_env(&pool, "rls-a", 3).await;
    let env_b = setup_test_env(&pool, "rls-b", 3).await;

    // Tenant A обновляет свой прогресс
    let resp_a = post_progress(
        pool.clone(),
        &env_a.token,
        env_a.lesson_ids[0],
        json!({"node_id": env_a.lesson_ids[0], "status": "completed"}),
    )
    .await;
    assert_eq!(resp_a.status(), StatusCode::OK);

    // Tenant B пытается обновить прогресс на свой урок (должно работать)
    let resp_b = post_progress(
        pool.clone(),
        &env_b.token,
        env_b.lesson_ids[0],
        json!({"node_id": env_b.lesson_ids[0], "status": "completed"}),
    )
    .await;
    assert_eq!(resp_b.status(), StatusCode::OK);

    // Критическая проверка: RLS не должен допускать кросс-тенантного влияния
    // Прогресс курса у A должен быть 1/3, у B — 1/3, независимо
    let data_a = extract_data(resp_a).await;
    let progress_a = data_a["course_progress"].as_f64().unwrap();

    let data_b = extract_data(resp_b).await;
    let progress_b = data_b["course_progress"].as_f64().unwrap();

    assert!((progress_a - 1.0 / 3.0).abs() < 0.001, "Tenant A progress was {progress_a}");
    assert!((progress_b - 1.0 / 3.0).abs() < 0.001, "Tenant B progress was {progress_b}");

    // Дополнительная проверка: курс из Tenant A не виден для Tenant B
    let router = create_router(pool, JwtConfig::default());
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/courses/{}", env_a.course_id))
        .header(http::header::AUTHORIZATION, format!("Bearer {}", env_b.token))
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(get_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}