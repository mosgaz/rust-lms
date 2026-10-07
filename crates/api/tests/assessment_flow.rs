// crates/api/tests/assessment_flow.rs
//! Интеграционные тесты для Assessments Engine (Этап 10).
//!
//! Покрывает полный сценарий: создание вопросов → начало попытки →
//! отправка ответов → завершение с подсчётом баллов → обновление прогресса.
//!
//! Требования для запуска:
//! - Запущенный PostgreSQL с применёнными миграциями
//! - Переменная окружения `DATABASE_URL` указывает на тестовую БД
//!
//! Запуск: `cargo test -p rust-lms-api --test assessment_flow`

use chrono::Utc;
use rust_lms_api::database::{
    AttemptRepository, QuestionRepository, ScoredAnswer,
};
use rust_lms_api::services::{ProgressService, ScoringEngine};
use rust_lms_shared::models::attempt::AttemptStatus;
use rust_lms_shared::models::lesson_progress::LessonStatus;
use rust_lms_shared::models::question::{CreateQuestionRequest, QuestionType};
use rust_lms_shared::{
    AttemptId, CourseId, NodeId, QuestionId, TenantId, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Создаёт тестовый тенант и возвращает его идентификатор.
async fn create_test_tenant(pool: &PgPool) -> TenantId {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, name, slug)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(id)
    .bind("Test Tenant")
    .bind(format!("test-{}", &id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("Failed to create test tenant");

    TenantId(id)
}

/// Создаёт тестового пользователя и возвращает его идентификатор.
async fn create_test_user(pool: &PgPool, tenant_id: TenantId) -> UserId {
    let identity_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Создаём глобальную личность
    sqlx::query(
        r#"
        INSERT INTO identities (id, email, password_hash)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(identity_id)
    .bind(format!("test-{}@example.com", &user_id.to_string()[..8]))
    .bind("$argon2id$fake_hash_for_testing")
    .execute(pool)
    .await
    .expect("Failed to create test identity");

    // Создаём привязку к тенанту
    sqlx::query(
        r#"
        INSERT INTO users (id, identity_id, tenant_id, is_active)
        VALUES ($1, $2, $3, true)
        "#,
    )
    .bind(user_id)
    .bind(identity_id)
    .bind(tenant_id.0)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    UserId(user_id)
}

/// Создаёт тестовый курс и возвращает его идентификатор.
async fn create_test_course(pool: &PgPool, tenant_id: TenantId) -> CourseId {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO courses (id, tenant_id, title, status)
        VALUES ($1, $2, $3, 'draft')
        "#,
    )
    .bind(id)
    .bind(tenant_id.0)
    .bind("Test Course: Rust Basics")
    .execute(pool)
    .await
    .expect("Failed to create test course");

    CourseId(id)
}

/// Создаёт тестовый узел (урок) и возвращает его идентификатор.
async fn create_test_lesson(
    pool: &PgPool,
    tenant_id: TenantId,
    course_id: CourseId,
) -> NodeId {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO nodes (id, tenant_id, course_id, node_type, title, path, weight)
        VALUES ($1, $2, $3, 'lesson', $4, $5, 1.0)
        "#,
    )
    .bind(id)
    .bind(tenant_id.0)
    .bind(course_id.0)
    .bind("Test Lesson")
    .bind(format!("{}.{}", Uuid::new_v4(), id))
    .execute(pool)
    .await
    .expect("Failed to create test lesson");

    NodeId(id)
}

/// Устанавливает RLS-контекст для текущего соединения.
async fn set_rls_context(pool: &PgPool, tenant_id: TenantId) {
    sqlx::query(&format!(
        "SET LOCAL app.current_tenant_id = '{}'",
        tenant_id.0
    ))
    .execute(pool)
    .await
    .expect("Failed to set RLS context");
}

/// Тест 10.1–10.3: Создание вопросов разных типов.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_create_questions_of_different_types(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let repo = QuestionRepository::new(pool.clone());

    set_rls_context(&pool, tenant_id).await;

    // Вопрос с выбором ответа (MultipleChoice)
    let mc_question = CreateQuestionRequest {
        title: "Какой язык компилируется в WebAssembly?".to_string(),
        description: Some("Выберите один вариант".to_string()),
        question_type: QuestionType::MultipleChoice,
        options: Some(vec![
            rust_lms_shared::models::question::AnswerOption {
                index: 0,
                text: "Python".to_string(),
            },
            rust_lms_shared::models::question::AnswerOption {
                index: 1,
                text: "Rust".to_string(),
            },
            rust_lms_shared::models::question::AnswerOption {
                index: 2,
                text: "Ruby".to_string(),
            },
        ]),
        correct_answer: Some("1".to_string()),
        points: 10,
        order: 1,
    };

    let created_mc = repo
        .create(tenant_id, course_id, &mc_question)
        .await
        .expect("Failed to create MultipleChoice question");

    assert_eq!(created_mc.title, "Какой язык компилируется в WebAssembly?");
    assert_eq!(created_mc.points, 10);

    // Вопрос True/False
    let tf_question = CreateQuestionRequest {
        title: "Rust имеет сборщик мусора.".to_string(),
        description: None,
        question_type: QuestionType::TrueFalse,
        options: None,
        correct_answer: Some("false".to_string()),
        points: 5,
        order: 2,
    };

    let created_tf = repo
        .create(tenant_id, course_id, &tf_question)
        .await
        .expect("Failed to create TrueFalse question");

    assert_eq!(created_tf.question_type, QuestionType::TrueFalse);

    // Вопрос с коротким ответом (ShortAnswer)
    let sa_question = CreateQuestionRequest {
        title: "Как называется система управления пакетами в Rust?".to_string(),
        description: None,
        question_type: QuestionType::ShortAnswer,
        options: None,
        correct_answer: Some("cargo".to_string()),
        points: 10,
        order: 3,
    };

    let created_sa = repo
        .create(tenant_id, course_id, &sa_question)
        .await
        .expect("Failed to create ShortAnswer question");

    assert_eq!(created_sa.question_type, QuestionType::ShortAnswer);

    // Проверяем, что все вопросы возвращаются списком
    let all_questions = repo
        .list_by_course(tenant_id, course_id)
        .await
        .expect("Failed to list questions");

    assert_eq!(all_questions.len(), 3);
}

/// Тест 10.4–10.5: Начало попытки, отправка ответов и подсчёт баллов.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_attempt_scoring_flow(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let user_id = create_test_user(&pool, tenant_id).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let question_repo = QuestionRepository::new(pool.clone());
    let attempt_repo = AttemptRepository::new(pool.clone());

    set_rls_context(&pool, tenant_id).await;

    // Создаём два вопроса
    let q1 = question_repo
        .create(
            tenant_id,
            course_id,
            &CreateQuestionRequest {
                title: "2 + 2 = ?".to_string(),
                description: None,
                question_type: QuestionType::ShortAnswer,
                options: None,
                correct_answer: Some("4".to_string()),
                points: 10,
                order: 1,
            },
        )
        .await
        .expect("Failed to create question 1");

    let q2 = question_repo
        .create(
            tenant_id,
            course_id,
            &CreateQuestionRequest {
                title: "Rust — это язык с GC?".to_string(),
                description: None,
                question_type: QuestionType::TrueFalse,
                options: None,
                correct_answer: Some("false".to_string()),
                points: 10,
                order: 2,
            },
        )
        .await
        .expect("Failed to create question 2");

    // Начинаем попытку
    let attempt = attempt_repo
        .create(tenant_id, user_id, course_id, Some(600))
        .await
        .expect("Failed to start attempt");

    assert_eq!(attempt.status, AttemptStatus::InProgress);
    assert_eq!(attempt.attempt_number, 1);

    // Формируем ответы и проверяем через ScoringEngine
    let questions = question_repo
        .list_by_course(tenant_id, course_id)
        .await
        .expect("Failed to list questions");

    let answers = vec![
        (q1.id, "4".to_string()),      // Правильный ответ
        (q2.id, "false".to_string()),   // Правильный ответ
    ];

    let scoring = ScoringEngine::new().score_attempt(&questions, &answers, 0.7);

    assert!(scoring.passed, "Тест должен быть сдан при двух правильных ответах");
    assert!(
        (scoring.total_score - 1.0).abs() < f64::EPSILON,
        "Балл должен быть 1.0 при всех правильных ответах"
    );

    // Сохраняем ответы в БД
    let scored_answers: Vec<ScoredAnswer> = scoring
        .results
        .iter()
        .map(|qr| {
            let answer_text = answers
                .iter()
                .find(|(qid, _)| *qid == qr.question_id)
                .map(|(_, text)| text.clone())
                .unwrap_or_default();

            ScoredAnswer {
                question_id: qr.question_id,
                answer_text,
                is_correct: qr.scoring.is_correct,
                points_earned: qr.scoring.points_earned,
                explanation: qr.scoring.explanation.clone(),
            }
        })
        .collect();

    attempt_repo
        .save_answers(tenant_id, attempt.id, &scored_answers)
        .await
        .expect("Failed to save answers");

    // Завершаем попытку
    let completed = attempt_repo
        .complete(tenant_id, attempt.id, scoring.total_score, scoring.passed, 120)
        .await
        .expect("Failed to complete attempt");

    assert_eq!(completed.status, AttemptStatus::Completed);
    assert_eq!(completed.passed, Some(true));
    assert!((completed.score.unwrap() - 1.0).abs() < f64::EPSILON);

    // Проверяем, что ответы сохранились в БД
    let saved_answers = attempt_repo
        .get_answers_for_attempt(tenant_id, attempt.id)
        .await
        .expect("Failed to get answers for attempt");

    assert_eq!(saved_answers.len(), 2, "Должно быть 2 сохранённых ответа");
    assert!(saved_answers.iter().all(|a| a.is_correct == Some(true)));
}

/// Тест 10.5: Подсчёт баллов при частичном правильном ответе.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_scoring_partial_correct(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let question_repo = QuestionRepository::new(pool.clone());

    set_rls_context(&pool, tenant_id).await;

    // Создаём два вопроса по 10 баллов каждый
    let q1 = question_repo
        .create(
            tenant_id,
            course_id,
            &CreateQuestionRequest {
                title: "Столица Франции?".to_string(),
                description: None,
                question_type: QuestionType::ShortAnswer,
                options: None,
                correct_answer: Some("париж".to_string()),
                points: 10,
                order: 1,
            },
        )
        .await
        .unwrap();

    let q2 = question_repo
        .create(
            tenant_id,
            course_id,
            &CreateQuestionRequest {
                title: "Столица Германии?".to_string(),
                description: None,
                question_type: QuestionType::ShortAnswer,
                options: None,
                correct_answer: Some("берлин".to_string()),
                points: 10,
                order: 2,
            },
        )
        .await
        .unwrap();

    let questions = vec![q1.clone(), q2.clone()];

    // Один правильный, один неправильный
    let answers = vec![
        (q1.id, "париж".to_string()),   // Правильный
        (q2.id, "мюнхен".to_string()),   // Неправильный
    ];

    let scoring = ScoringEngine::new().score_attempt(&questions, &answers, 0.7);

    assert!(
        !scoring.passed,
        "Тест не должен быть сдан при 50% правильных (порог 70%)"
    );
    assert!(
        (scoring.total_score - 0.5).abs() < f64::EPSILON,
        "Балл должен быть 0.5 при одном правильном из двух"
    );
    assert_eq!(scoring.total_points_earned, 10);
    assert_eq!(scoring.total_points_possible, 20);
}

/// Тест 10.4: Защита от превышения лимита попыток.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_attempt_limit_enforcement(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let user_id = create_test_user(&pool, tenant_id).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let attempt_repo = AttemptRepository::new(pool.clone());

    set_rls_context(&pool, tenant_id).await;

    // Создаём 10 попыток (максимум) и завершаем их
    for i in 1..=10 {
        let attempt = attempt_repo
            .create(tenant_id, user_id, course_id, None)
            .await
            .unwrap_or_else(|_| panic!("Failed to create attempt {}", i));

        attempt_repo
            .complete(tenant_id, attempt.id, 0.5, false, 60)
            .await
            .expect("Failed to complete attempt");
    }

    // 11-я попытка должна быть отклонена
    let result = attempt_repo
        .create(tenant_id, user_id, course_id, None)
        .await;

    assert!(
        result.is_err(),
        "11-я попытка должна быть отклонена"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Maximum attempts"),
        "Ошибка должна содержать 'Maximum attempts', получено: {}",
        err_msg
    );
}

/// Тест 10.7: Интеграция с прогрессом обучения.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_progress_integration_on_pass(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let user_id = create_test_user(&pool, tenant_id).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let node_id = create_test_lesson(&pool, tenant_id, course_id).await;

    let lesson_progress_repo =
        rust_lms_api::database::LessonProgressRepository::new(pool.clone());
    let progress_service = ProgressService::new(lesson_progress_repo);

    set_rls_context(&pool, tenant_id).await;

    // Имитируем успешную сдачу теста: обновляем прогресс урока
    let result = progress_service
        .update_lesson_progress(
            tenant_id,
            user_id,
            node_id,
            Some(LessonStatus::Completed),
            Some(0.95),
            Some(300),
            None,
            None,
        )
        .await
        .expect("Failed to update lesson progress");

    assert_eq!(
        result.lesson_progress.status,
        LessonStatus::Completed,
        "Статус урока должен быть Completed после успешной сдачи"
    );
    assert!(
        (result.lesson_progress.score.unwrap() - 0.95).abs() < f64::EPSILON,
        "Балл урока должен совпадать с баллом теста"
    );
}

/// Тест 10.4: История попыток пользователя.
#[sqlx::test(migrations = "crates/api/migrations")]
async fn test_attempt_history(pool: PgPool) {
    let tenant_id = create_test_tenant(&pool).await;
    let user_id = create_test_user(&pool, tenant_id).await;
    let course_id = create_test_course(&pool, tenant_id).await;
    let attempt_repo = AttemptRepository::new(pool.clone());

    set_rls_context(&pool, tenant_id).await;

    // Создаём 3 попытки с разными результатами
    for (i, (score, passed)) in [(0.5, false), (0.8, true), (1.0, true)].iter().enumerate() {
        let attempt = attempt_repo
            .create(tenant_id, user_id, course_id, None)
            .await
            .unwrap_or_else(|_| panic!("Failed to create attempt {}", i + 1));

        attempt_repo
            .complete(tenant_id, attempt.id, *score, *passed, 60 * (i as i32 + 1))
            .await
            .expect("Failed to complete attempt");
    }

    // Получаем историю
    let history = attempt_repo
        .list_by_user_and_course(tenant_id, user_id, course_id)
        .await
        .expect("Failed to list attempts");

    assert_eq!(history.len(), 3, "Должно быть 3 попытки в истории");

    // Проверяем, что попытки отсортированы по дате (последние первыми)
    assert!(history[0].started_at >= history[1].started_at);
    assert!(history[1].started_at >= history[2].started_at);

    // Проверяем номера попыток
    let attempt_numbers: Vec<i32> = history.iter().map(|a| a.attempt_number).collect();
    assert!(attempt_numbers.contains(&1));
    assert!(attempt_numbers.contains(&2));
    assert!(attempt_numbers.contains(&3));
}