// crates/api/src/database/repositories/lesson_progress.rs
//! Репозиторий для управления прогрессом обучения (LessonProgress).

use chrono::Utc;
use rust_lms_shared::{
    CompletionCriteria, CompletionMode, CompletionRule, CourseId, CourseProgressSummary,
    LessonProgress, LessonProgressId, LessonStatus, NodeId, TenantId, UserId,
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
    /// Курс уже завершён.
    #[error("Course already completed")]
    CourseAlreadyCompleted,
    /// Недостаточно прав для выполнения операции.
    #[error("Insufficient permissions")]
    Forbidden,
}

/// Результат обновления прогресса урока.
#[derive(Debug, Clone)]
pub struct LessonProgressUpdateResult {
    /// Идентификатор курса.
    pub course_id: CourseId,
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

    /// Проверяет, является ли пользователь инструктором или администратором тенанта.
    pub async fn is_instructor_or_admin(&self, tenant_id: TenantId, user_id: UserId) -> Result<bool, sqlx::Error> {
        let is_allowed: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                SELECT 1 FROM users WHERE id = $1 AND tenant_id = $2 AND role IN ('instructor', 'admin')
            )"#,
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .fetch_one(&self.pool)
        .await?;
        Ok(is_allowed)
    }

    fn check_completion_criteria(
        criteria: &CompletionCriteria,
        course_progress: f64,
        avg_quiz_score: Option<f64>,
        completed_node_ids: &[Uuid],
        total_lessons: i32,
        completed_lessons: i32,
    ) -> bool {
        let results: Vec<bool> = criteria
            .rules
            .iter()
            .map(|rule| match rule {
                CompletionRule::MinProgress { value } => course_progress >= *value,
                CompletionRule::MinAvgQuizScore { value } => {
                    avg_quiz_score.map_or(false, |s| s >= *value)
                }
                CompletionRule::RequiredNodes { node_ids } => {
                    node_ids.iter().all(|id| completed_node_ids.contains(&id.0))
                }
                CompletionRule::AllLessonsCompleted => completed_lessons == total_lessons,
            })
            .collect();

        match criteria.mode {
            CompletionMode::AllOf => results.iter().all(|&r| r),
            CompletionMode::AnyOf => results.iter().any(|&r| r),
        }
    }

    /// Создает или обновляет прогресс урока, пересчитывает прогресс курса и проверяет критерии завершения.
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

        let new_time_spent = time_spent_seconds
            .unwrap_or_else(|| existing_progress.as_ref().map(|p| p.4).unwrap_or(0));
        let new_last_position = last_position
            .unwrap_or_else(|| existing_progress.as_ref().map(|p| p.6).unwrap_or(0));
        let new_client_modified_at =
            client_modified_at.or_else(|| existing_progress.as_ref().and_then(|p| p.8));

        let mut new_attempt_count = existing_progress.as_ref().map(|p| p.5).unwrap_or(0);
        if is_quiz && new_status == LessonStatus::Completed {
            let was_completed = existing_progress
                .as_ref()
                .map(|p| p.1 == "completed")
                .unwrap_or(false);
            if !was_completed {
                new_attempt_count += 1;
            }
        }

        let now = Utc::now();
        let final_completed_at = if new_status == LessonStatus::Completed {
            if existing_progress
                .as_ref()
                .map(|p| p.1 == "completed")
                .unwrap_or(false)
            {
                existing_progress.as_ref().and_then(|p| p.7)
            } else {
                Some(now)
            }
        } else {
            None
        };

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

        let old_status_str = existing_progress
            .as_ref()
            .map(|p| p.1.as_str())
            .unwrap_or("not_started");
        let new_status_str = new_status.as_str();

        let delta = if new_status_str == "completed" && old_status_str != "completed" {
            node_weight
        } else if new_status_str != "completed" && old_status_str == "completed" {
            -node_weight
        } else {
            0.0
        };

        current_completed_weight = (current_completed_weight + delta).max(0.0);

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
                "#,
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
            course_id,
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

    /// Получает детальный прогресс пользователя по конкретному курсу (все уроки, включая не начатые).
    pub async fn get_user_course_progress(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        course_id: CourseId,
    ) -> Result<Vec<LessonProgress>, sqlx::Error> {
        let is_enrolled: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (SELECT 1 FROM course_enrollments WHERE user_id = $1 AND course_id = $2)"#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .fetch_one(&self.pool)
        .await?;

        if !is_enrolled {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT 
                $3 as "tenant_id: uuid",
                $1 as "user_id: uuid",
                n.id as "node_id: uuid",
                lp.id as "id: uuid",
                lp.status as "status: varchar",
                lp.score,
                lp.passed,
                lp.time_spent_seconds,
                lp.attempt_count,
                lp.last_position,
                lp.completed_at,
                lp.client_modified_at,
                lp.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                lp.updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
            FROM nodes n
            LEFT JOIN lesson_progress lp ON n.id = lp.node_id AND lp.user_id = $1
            WHERE n.course_id = $2 AND n.tenant_id = $3 AND n.is_archived = FALSE AND n.node_type = 'lesson'
            "#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Option<Uuid> = row.get("id");
            let status: Option<String> = row.get("status");

            result.push(LessonProgress {
                id: LessonProgressId(id.unwrap_or_else(Uuid::new_v4)),
                tenant_id: TenantId(row.get("tenant_id")),
                user_id: UserId(row.get("user_id")),
                node_id: NodeId(row.get("node_id")),
                status: status
                    .map(|s| s.parse().unwrap_or(LessonStatus::NotStarted))
                    .unwrap_or(LessonStatus::NotStarted),
                score: row.get("score"),
                passed: row.get("passed"),
                time_spent_seconds: row.get::<Option<i32>, _>("time_spent_seconds").unwrap_or(0),
                attempt_count: row.get::<Option<i32>, _>("attempt_count").unwrap_or(0),
                last_position: row.get::<Option<i32>, _>("last_position").unwrap_or(0),
                completed_at: row.get("completed_at"),
                client_modified_at: row.get("client_modified_at"),
                created_at: row
                    .get::<Option<chrono::DateTime<Utc>>, _>("created_at")
                    .unwrap_or_else(Utc::now),
                updated_at: row
                    .get::<Option<chrono::DateTime<Utc>>, _>("updated_at")
                    .unwrap_or_else(Utc::now),
            });
        }
        Ok(result)
    }

    /// Получает прогресс всех студентов курса с пагинацией.
    pub async fn get_course_students_progress(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CourseProgressSummary>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                $1 as "course_id: uuid",
                ce.user_id as "user_id: uuid",
                ce.progress,
                ce.status,
                ce.completed_at,
                COALESCE(stats.completed_lessons, 0) as "completed_lessons_count: i32",
                COALESCE(stats.total_lessons, 0) as "total_lessons_count: i32",
                ce.completed_lessons_weight,
                COALESCE((SELECT SUM(COALESCE((metadata->>'weight')::numeric, 1.0)) FROM nodes WHERE course_id = $1 AND is_archived = FALSE AND node_type = 'lesson'), 0.0) as "total_lessons_weight: f64"
            FROM course_enrollments ce
            JOIN courses c ON ce.course_id = c.id
            LEFT JOIN (
                SELECT 
                    lp.user_id,
                    COUNT(n.id) FILTER (WHERE n.node_type = 'lesson' AND lp.status = 'completed') as completed_lessons,
                    COUNT(n.id) FILTER (WHERE n.node_type = 'lesson') as total_lessons
                FROM lesson_progress lp
                JOIN nodes n ON lp.node_id = n.id
                WHERE n.course_id = $1 
                GROUP BY lp.user_id
            ) stats ON ce.user_id = stats.user_id
            WHERE ce.course_id = $1 AND c.tenant_id = $2
            ORDER BY ce.enrolled_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CourseProgressSummary {
                course_id: CourseId(row.get("course_id")),
                user_id: UserId(row.get("user_id")),
                progress: row.get::<f64, _>("progress"),
                status: row.get("status"),
                completed_at: row.get("completed_at"),
                completed_lessons_count: row.get("completed_lessons_count"),
                total_lessons_count: row.get("total_lessons_count"),
                completed_lessons_weight: row.get("completed_lessons_weight"),
                total_lessons_weight: row.get("total_lessons_weight"),
            })
            .collect())
    }

    /// Принудительный пересчёт прогресса для всех зачисленных студентов курса.
    pub async fn recalculate_course_progress(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<usize, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let course_data = sqlx::query(
            r#"
            SELECT 
                completion_criteria as "completion_criteria: serde_json::Value",
                (SELECT COALESCE(SUM(COALESCE((metadata->>'weight')::numeric, 1.0)), 0.0) 
                 FROM nodes WHERE course_id = $1 AND tenant_id = $2 AND is_archived = FALSE AND node_type = 'lesson') as "total_weight: f64"
            FROM courses WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_one(&mut *tx)
        .await?;

        let total_weight = course_data.get::<f64, _>("total_weight").max(0.0001);
        let criteria = course_data
            .get::<Option<serde_json::Value>, _>("completion_criteria")
            .and_then(|json| CompletionCriteria::from_json(&json).ok());

        let student_stats = sqlx::query(
            r#"
            SELECT 
                ce.user_id as "user_id: uuid",
                ce.status as "current_status: varchar",
                ce.completed_at,
                ce.completed_lessons_weight as "current_weight: f64",
                ce.progress as "current_progress: f64",
                COALESCE(SUM(CASE WHEN lp.status = 'completed' THEN COALESCE((n.metadata->>'weight')::numeric, 1.0) ELSE 0.0 END), 0.0) as "calculated_weight: f64",
                COALESCE(AVG(CASE WHEN n.metadata->'quiz' IS NOT NULL AND lp.status = 'completed' THEN lp.score END), 0.0) as "avg_quiz_score: f64",
                COALESCE(array_agg(n.id) FILTER (WHERE lp.status = 'completed'), ARRAY[]::uuid[]) as "completed_node_ids: uuid[]",
                COUNT(n.id) FILTER (WHERE n.node_type = 'lesson' AND lp.status = 'completed') as "completed_lessons: i32",
                COUNT(n.id) FILTER (WHERE n.node_type = 'lesson') as "total_lessons: i32"
            FROM course_enrollments ce
            LEFT JOIN nodes n ON n.course_id = ce.course_id AND n.tenant_id = ce.tenant_id AND n.is_archived = FALSE AND n.node_type = 'lesson'
            LEFT JOIN lesson_progress lp ON n.id = lp.node_id AND lp.user_id = ce.user_id
            WHERE ce.course_id = $1
            GROUP BY ce.user_id, ce.status, ce.completed_at, ce.completed_lessons_weight, ce.progress
            "#,
        )
        .bind(course_id.0)
        .fetch_all(&mut *tx)
        .await?;

        let mut updated_count = 0;

        for row in student_stats {
            let user_id = UserId(row.get::<Uuid, _>("user_id"));
            let calculated_weight: f64 = row.get("calculated_weight");
            let calculated_progress = (calculated_weight / total_weight).min(1.0).max(0.0);
            
            let avg_quiz_score: Option<f64> = row.get("avg_quiz_score");
            let completed_node_ids: Vec<Uuid> = row.get("completed_node_ids");
            let total_lessons: i32 = row.get("total_lessons");
            let completed_lessons: i32 = row.get("completed_lessons");
            
            let current_status: String = row.get("current_status");
            let current_completed_at: Option<chrono::DateTime<Utc>> = row.get("completed_at");
            let current_weight: f64 = row.get("current_weight");
            let current_progress: f64 = row.get("current_progress");

            let should_complete = if let Some(ref crit) = criteria {
                Self::check_completion_criteria(
                    crit,
                    calculated_progress,
                    avg_quiz_score,
                    &completed_node_ids,
                    total_lessons,
                    completed_lessons,
                )
            } else {
                calculated_progress >= 1.0
            };

            let new_status = if should_complete && current_status != "completed" {
                "completed".to_string()
            } else {
                current_status.clone()
            };

            let new_completed_at = if new_status == "completed" && current_completed_at.is_none() {
                Some(Utc::now())
            } else {
                current_completed_at
            };

            let weight_changed = (calculated_weight - current_weight).abs() > 0.0001;
            let progress_changed = (calculated_progress - current_progress).abs() > 0.0001;
            let status_changed = new_status != current_status;
            let date_changed = new_completed_at != current_completed_at;

            if status_changed || date_changed || weight_changed || progress_changed {
                sqlx::query(
                    r#"
                    UPDATE course_enrollments
                    SET progress = $1, completed_lessons_weight = $2, status = $3, completed_at = $4
                    WHERE user_id = $5 AND course_id = $6
                    "#,
                )
                .bind(calculated_progress)
                .bind(calculated_weight)
                .bind(&new_status)
                .bind(new_completed_at)
                .bind(user_id.0)
                .bind(course_id.0)
                .execute(&mut *tx)
                .await?;
                updated_count += 1;
            }
        }

        tx.commit().await?;
        Ok(updated_count)
    }
}