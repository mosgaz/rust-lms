// crates/api/src/database/repositories/lesson_progress.rs
//! Репозиторий для управления прогрессом обучения (LessonProgress).
//!
//! Обеспечивает транзакционное обновление прогресса урока с автоматическим
//! пересчетом прогресса курса и проверкой критериев завершения.
//!
//! Примечание: Используется `sqlx::query` (без `!`) для возможности офлайн-компиляции
//! без требования наличия `DATABASE_URL` или кэша `.sqlx`.

use chrono::Utc;
use rust_lms_shared::{
    CompletionCriteria, CompletionMode, CompletionRule, CourseId, LessonProgress, LessonProgressId,
    LessonStatus, NodeId, TenantId, UserId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Ошибки репозитория прогресса обучения.
#[derive(Debug, thiserror::Error)]
pub enum LessonProgressRepositoryError {
    /// Ошибка базы данных.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// Узел не найден.
    #[error("Node not found")]
    NodeNotFound,
    /// Узел архивирован.
    #[error("Node is archived")]
    NodeArchived,
    /// Пользователь не зачислен в курс.
    #[error("User not enrolled in course")]
    NotEnrolled,
    /// Курс уже завершён, обновление прогресса запрещено.
    #[error("Course already completed")]
    CourseAlreadyCompleted,
}

/// Результат обновления прогресса урока.
#[derive(Debug, Clone)]
pub struct LessonProgressUpdateResult {
    /// Обновлённый прогресс урока.
    pub lesson_progress: LessonProgress,
    /// Текущий прогресс курса (0.0–1.0).
    pub course_progress: f64,
    /// Статус курса.
    pub course_status: String,
    /// Было ли инициировано завершение курса.
    pub completion_triggered: bool,
}

/// Репозиторий прогресса обучения.
#[derive(Debug, Clone)]
pub struct LessonProgressRepository {
    pool: PgPool,
}

impl LessonProgressRepository {
    /// Создает новый экземпляр репозитория.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Проверяет выполнение критериев завершения курса.
    fn check_completion_criteria(
        criteria: &CompletionCriteria,
        course_progress: f64,
        avg_quiz_score: Option<f64>,
        completed_node_ids: &[Uuid],
        total_lessons: i32,
        completed_lessons: i32,
    ) -> bool {
        let results: Vec<bool> = criteria.rules.iter().map(|rule| {
            match rule {
                CompletionRule::MinProgress { value } => course_progress >= *value,
                CompletionRule::MinAvgQuizScore { value } => {
                    avg_quiz_score.map_or(false, |s| s >= *value)
                }
                CompletionRule::RequiredNodes { node_ids } => {
                    node_ids.iter().all(|id| completed_node_ids.contains(&id.0))
                }
                CompletionRule::AllLessonsCompleted => completed_lessons == total_lessons,
            }
        }).collect();

        match criteria.mode {
            CompletionMode::AllOf => results.iter().all(|&r| r),
            CompletionMode::AnyOf => results.iter().any(|&r| r),
        }
    }

    /// Создает или обновляет прогресс урока, пересчитывает прогресс курса и проверяет критерии завершения.
    ///
    /// Вся операция выполняется в одной транзакции с блокировкой `SELECT FOR UPDATE`
    /// на строке `course_enrollments` для защиты от race conditions.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_and_recalculate(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        node_id: NodeId,
        status: Option<LessonStatus>,
        score: Option<f64>,
        time_spent_seconds: Option<i32>,
        last_position: Option<i32>,
        client_modified_at: Option<chrono::DateTime<Utc>>,
    ) -> Result<LessonProgressUpdateResult, LessonProgressRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Информация об узле
        let node_info = sqlx::query(
            r#"SELECT course_id, metadata, is_archived FROM nodes WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(node_id.0)
        .bind(tenant_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .map(|row| {
            (
                row.get::<Uuid, _>("course_id"),
                row.get::<serde_json::Value, _>("metadata"),
                row.get::<bool, _>("is_archived"),
            )
        })
        .ok_or(LessonProgressRepositoryError::NodeNotFound)?;

        if node_info.2 {
            return Err(LessonProgressRepositoryError::NodeArchived);
        }

        let course_id = CourseId(node_info.0);
        let metadata = node_info.1;
        let node_weight = metadata.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let is_quiz = metadata.get("quiz").is_some() || metadata.get("assessment").is_some();
        let passing_score = metadata
            .get("quiz")
            .and_then(|q| q.get("passing_score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        // 2. Блокируем и проверяем зачисление (P0: NotEnrolled, P1: CourseAlreadyCompleted)
        let enrollment = sqlx::query(
            r#"SELECT status, completed_lessons_weight FROM course_enrollments WHERE user_id = $1 AND course_id = $2 FOR UPDATE"#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(LessonProgressRepositoryError::NotEnrolled)?;

        let current_status = enrollment.get::<String, _>("status");
        let mut current_completed_weight: f64 = enrollment.get::<f64, _>("completed_lessons_weight");

        if current_status == "completed" {
            return Err(LessonProgressRepositoryError::CourseAlreadyCompleted);
        }

        // 3. Текущий прогресс урока
        let existing_progress = sqlx::query(
            r#"SELECT id, status, score, passed, time_spent_seconds, attempt_count, last_position, completed_at, client_modified_at, created_at 
               FROM lesson_progress WHERE user_id = $1 AND node_id = $2"#,
        )
        .bind(user_id.0)
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .map(|row| {
            (
                row.get::<Uuid, _>("id"),
                row.get::<String, _>("status"),
                row.get::<Option<f64>, _>("score"),
                row.get::<Option<bool>, _>("passed"),
                row.get::<i32, _>("time_spent_seconds"),
                row.get::<i32, _>("attempt_count"),
                row.get::<i32, _>("last_position"),
                row.get::<Option<chrono::DateTime<Utc>>, _>("completed_at"),
                row.get::<Option<chrono::DateTime<Utc>>, _>("client_modified_at"),
                row.get::<chrono::DateTime<Utc>, _>("created_at"),
            )
        });

        // 4. Вычисление новых значений
        let new_status = status.unwrap_or_else(|| {
            existing_progress
                .as_ref()
                .and_then(|p| p.1.parse().ok())
                .unwrap_or(LessonStatus::NotStarted)
        });

        let new_score = score.or_else(|| existing_progress.as_ref().and_then(|p| p.2));
        let new_passed = if let Some(s) = new_score {
            Some(s >= passing_score)
        } else {
            existing_progress.as_ref().and_then(|p| p.3)
        };

        let new_time_spent = time_spent_seconds.unwrap_or_else(|| existing_progress.as_ref().map(|p| p.4).unwrap_or(0));
        let new_last_position = last_position.unwrap_or_else(|| existing_progress.as_ref().map(|p| p.6).unwrap_or(0));
        let new_client_modified_at = client_modified_at.or_else(|| existing_progress.as_ref().and_then(|p| p.8));

        let mut new_attempt_count = existing_progress.as_ref().map(|p| p.5).unwrap_or(0);
        if is_quiz && new_status == LessonStatus::Completed {
            let was_completed = existing_progress.as_ref().map(|p| p.1 == "completed").unwrap_or(false);
            if !was_completed {
                new_attempt_count += 1;
            }
        }

        let now = Utc::now();
        
        // P1 Fix: Сбрасываем completed_at в NULL, если статус больше не completed
        let final_completed_at = if new_status == LessonStatus::Completed {
            if existing_progress.as_ref().map(|p| p.1 == "completed").unwrap_or(false) {
                existing_progress.as_ref().and_then(|p| p.7)
            } else {
                Some(now)
            }
        } else {
            None
        };

        // 5. Upsert lesson_progress
        let progress_id = if let Some(ref ep) = existing_progress {
            let res = sqlx::query(
                r#"UPDATE lesson_progress
                   SET status = $1, score = $2, passed = $3, time_spent_seconds = $4,
                       attempt_count = $5, last_position = $6, completed_at = $7, client_modified_at = $8
                   WHERE id = $9 RETURNING id"#,
            )
            .bind(new_status.as_str())
            .bind(new_score)
            .bind(new_passed)
            .bind(new_time_spent)
            .bind(new_attempt_count)
            .bind(new_last_position)
            .bind(final_completed_at)
            .bind(new_client_modified_at)
            .bind(ep.0)
            .fetch_one(&mut *tx)
            .await?;
            res.get::<Uuid, _>("id")
        } else {
            let new_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO lesson_progress (id, tenant_id, user_id, node_id, status, score, passed, 
                   time_spent_seconds, attempt_count, last_position, completed_at, client_modified_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id"#,
            )
            .bind(new_id)
            .bind(tenant_id.0)
            .bind(user_id.0)
            .bind(node_id.0)
            .bind(new_status.as_str())
            .bind(new_score)
            .bind(new_passed)
            .bind(new_time_spent)
            .bind(new_attempt_count)
            .bind(new_last_position)
            .bind(final_completed_at)
            .bind(new_client_modified_at)
            .fetch_one(&mut *tx)
            .await?
            .get::<Uuid, _>("id")
        };

        // 6. Инкрементальный пересчёт веса (P1)
        let old_status_str = existing_progress.as_ref().map(|p| p.1.as_str()).unwrap_or("not_started");
        let new_status_str = new_status.as_str();

        let delta = if new_status_str == "completed" && old_status_str != "completed" {
            node_weight
        } else if new_status_str != "completed" && old_status_str == "completed" {
            -node_weight
        } else {
            0.0
        };

        current_completed_weight = (current_completed_weight + delta).max(0.0);

        // P0 Fix: COALESCE для веса узла, чтобы отсутствующий weight считался как 1.0
        let total_weight_row = sqlx::query(
            r#"SELECT COALESCE(SUM(COALESCE((metadata->>'weight')::numeric, 1.0)), 0.0) as total_weight
               FROM nodes WHERE course_id = $1 AND tenant_id = $2 AND is_archived = FALSE AND node_type = 'lesson'"#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_one(&mut *tx)
        .await?;
        let total_weight: f64 = total_weight_row.get::<f64, _>("total_weight").max(0.0001);
        
        let course_progress = (current_completed_weight / total_weight).min(1.0).max(0.0);

        // 7. Проверка критериев завершения (P0)
        let course_criteria = sqlx::query(
            r#"SELECT completion_criteria FROM courses WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .and_then(|row| row.get::<Option<serde_json::Value>, _>("completion_criteria"));

        let mut completion_triggered = false;
        let mut new_enrollment_status = current_status;

        // P2 Fix: Выполняем тяжёлый статистический запрос только если есть кастомные критерии
        let should_complete = if let Some(criteria_json) = course_criteria {
            let stats_row = sqlx::query(
                r#"
                SELECT
                    COUNT(n.id) FILTER (WHERE n.node_type = 'lesson')::int as total_lessons,
                    COUNT(n.id) FILTER (WHERE n.node_type = 'lesson' AND lp.status = 'completed')::int as completed_lessons,
                    AVG(lp.score) FILTER (WHERE n.metadata->'quiz' IS NOT NULL AND lp.status = 'completed') as avg_quiz_score,
                    COALESCE(array_agg(n.id) FILTER (WHERE lp.status = 'completed'), ARRAY[]::uuid[]) as completed_node_ids
                FROM nodes n
                LEFT JOIN lesson_progress lp ON n.id = lp.node_id AND lp.user_id = $1
                WHERE n.course_id = $2 AND n.tenant_id = $3 AND n.is_archived = FALSE AND n.node_type = 'lesson'
                "#
            )
            .bind(user_id.0)
            .bind(course_id.0)
            .bind(tenant_id.0)
            .fetch_one(&mut *tx)
            .await?;

            let total_lessons: i32 = stats_row.get::<i32, _>("total_lessons");
            let completed_lessons: i32 = stats_row.get::<i32, _>("completed_lessons");
            let avg_quiz_score: Option<f64> = stats_row.get::<Option<f64>, _>("avg_quiz_score");
            let completed_node_ids: Vec<Uuid> = stats_row.get::<Vec<Uuid>, _>("completed_node_ids");

            if let Ok(criteria) = CompletionCriteria::from_json(&criteria_json) {
                Self::check_completion_criteria(
                    &criteria,
                    course_progress,
                    avg_quiz_score,
                    &completed_node_ids,
                    total_lessons,
                    completed_lessons,
                )
            } else {
                course_progress >= 1.0
            }
        } else {
            course_progress >= 1.0
        };

        if should_complete {
            new_enrollment_status = "completed".to_string();
            completion_triggered = true;
        }

        // 8. Обновление course_enrollments
        sqlx::query(
            r#"UPDATE course_enrollments
               SET progress = $1, completed_lessons_weight = $2, status = $3,
                   completed_at = CASE WHEN $3 = 'completed' AND completed_at IS NULL THEN NOW() ELSE completed_at END
               WHERE user_id = $4 AND course_id = $5"#,
        )
        .bind(course_progress)
        .bind(current_completed_weight)
        .bind(&new_enrollment_status)
        .bind(user_id.0)
        .bind(course_id.0)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let created_at = existing_progress.as_ref().map(|p| p.9).unwrap_or(now);

        Ok(LessonProgressUpdateResult {
            lesson_progress: LessonProgress {
                id: LessonProgressId(progress_id),
                tenant_id,
                user_id,
                node_id,
                status: new_status,
                score: new_score,
                passed: new_passed,
                time_spent_seconds: new_time_spent,
                attempt_count: new_attempt_count,
                last_position: new_last_position,
                completed_at: final_completed_at,
                client_modified_at: new_client_modified_at,
                created_at,
                updated_at: now,
            },
            course_progress,
            course_status: new_enrollment_status,
            completion_triggered,
        })
    }
}