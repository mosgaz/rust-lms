// crates/api/src/database/repositories/attempt.rs
//! Репозиторий попыток (Attempt) прохождения тестов.
//!
//! Обеспечивает управление попытками: создание, сохранение ответов,
//! завершение с подсчётом баллов, история попыток.
//! Все операции учитывают RLS-изоляцию через `tenant_id`.

use chrono::{DateTime, Utc};
use rust_lms_shared::{
    Attempt, AttemptId, AttemptStatus, CourseId, TenantId, UserId,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Ошибки репозитория попыток.
#[derive(Debug, thiserror::Error)]
pub enum AttemptRepositoryError {
    /// Ошибка базы данных.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// Попытка не найдена.
    #[error("Attempt not found: {0}")]
    NotFound(AttemptId),
    /// Попытка уже завершена.
    #[error("Attempt already completed: {0}")]
    AlreadyCompleted(AttemptId),
    /// Попытка не в статусе "в процессе".
    #[error("Attempt is not in progress: {0}")]
    NotInProgress(AttemptId),
    /// Превышен лимит времени.
    #[error("Attempt timed out: {0}")]
    TimedOut(AttemptId),
    /// Ошибка сериализации/десериализации.
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// Неверный статус попытки в базе данных.
    #[error("Invalid attempt status in database: {0}")]
    InvalidStatus(String),
    /// Превышено максимальное количество попыток.
    #[error("Maximum attempts reached for course {course_id}: {max}")]
    MaxAttemptsReached {
        /// Идентификатор курса.
        course_id: CourseId,
        /// Максимальное количество попыток.
        max: i32,
    },
}

/// Репозиторий попыток.
#[derive(Debug, Clone)]
pub struct AttemptRepository {
    pool: PgPool,
}

impl AttemptRepository {
    /// Создаёт новый экземпляр репозитория.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новую попытку прохождения теста.
    ///
    /// Автоматически увеличивает `attempt_number` на основе количества
    /// предыдущих попыток пользователя по этому курсу.
    ///
    /// # Ошибки
    ///
    /// Возвращает `MaxAttemptsReached`, если достигнут лимит попыток.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        course_id: CourseId,
        time_limit_seconds: Option<i32>,
    ) -> Result<Attempt, AttemptRepositoryError> {
        // Определяем номер попытки (количество существующих + 1)
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt
            FROM attempts
            WHERE user_id = $1 AND course_id = $2 AND tenant_id = $3
            "#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = count_row.get("cnt");
        let attempt_number = (count + 1) as i32;

        // Проверяем лимит попыток (пока жёстко 10, позже можно брать из метаданных курса)
        const MAX_ATTEMPTS: i32 = 10;
        if attempt_number > MAX_ATTEMPTS {
            return Err(AttemptRepositoryError::MaxAttemptsReached {
                course_id,
                max: MAX_ATTEMPTS,
            });
        }

        let id = AttemptId(Uuid::new_v4());
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO attempts (
                id, tenant_id, user_id, course_id, status,
                started_at, time_limit_seconds, attempt_number
            )
            VALUES ($1, $2, $3, $4, 'in_progress', $5, $6, $7)
            RETURNING
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            "#,
        )
        .bind(id.0)
        .bind(tenant_id.0)
        .bind(user_id.0)
        .bind(course_id.0)
        .bind(now)
        .bind(time_limit_seconds)
        .bind(attempt_number)
        .fetch_one(&self.pool)
        .await?;

        Self::row_to_attempt(&row)
    }

    /// Получает попытку по идентификатору.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если попытка не существует или не принадлежит тенанту.
    pub async fn get_by_id(
        &self,
        tenant_id: TenantId,
        attempt_id: AttemptId,
    ) -> Result<Attempt, AttemptRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            FROM attempts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(attempt_id.0)
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AttemptRepositoryError::NotFound(attempt_id))?;

        Self::row_to_attempt(&row)
    }

    /// Завершает попытку с подсчётом баллов.
    ///
    /// Устанавливает статус `completed`, записывает балл и признак сдачи.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если попытка не существует.
    /// Возвращает `AlreadyCompleted`, если попытка уже завершена.
    /// Возвращает `TimedOut`, если превышен лимит времени.
    pub async fn complete(
        &self,
        tenant_id: TenantId,
        attempt_id: AttemptId,
        score: f64,
        passed: bool,
        time_spent_seconds: i32,
    ) -> Result<Attempt, AttemptRepositoryError> {
        // Получаем текущую попытку для проверки статуса
        let current = self.get_by_id(tenant_id, attempt_id).await?;

        if current.status != AttemptStatus::InProgress {
            return Err(AttemptRepositoryError::AlreadyCompleted(attempt_id));
        }

        // Проверяем лимит времени
        if let Some(limit) = current.time_limit_seconds {
            if time_spent_seconds > limit {
                return Err(AttemptRepositoryError::TimedOut(attempt_id));
            }
        }

        let now = Utc::now();

        let row = sqlx::query(
            r#"
            UPDATE attempts
            SET
                status = 'completed',
                completed_at = $1,
                score = $2,
                passed = $3,
                time_spent_seconds = $4
            WHERE id = $5 AND tenant_id = $6
            RETURNING
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(score)
        .bind(passed)
        .bind(time_spent_seconds)
        .bind(attempt_id.0)
        .bind(tenant_id.0)
        .fetch_one(&self.pool)
        .await?;

        Self::row_to_attempt(&row)
    }

    /// Помечает попытку как отменённую пользователем.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если попытка не существует.
    /// Возвращает `NotInProgress`, если попытка не в статусе "в процессе".
    pub async fn abandon(
        &self,
        tenant_id: TenantId,
        attempt_id: AttemptId,
    ) -> Result<Attempt, AttemptRepositoryError> {
        let current = self.get_by_id(tenant_id, attempt_id).await?;

        if current.status != AttemptStatus::InProgress {
            return Err(AttemptRepositoryError::NotInProgress(attempt_id));
        }

        let row = sqlx::query(
            r#"
            UPDATE attempts
            SET status = 'abandoned'
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            "#,
        )
        .bind(attempt_id.0)
        .bind(tenant_id.0)
        .fetch_one(&self.pool)
        .await?;

        Self::row_to_attempt(&row)
    }

    /// Возвращает историю попыток пользователя по курсу.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку базы данных при сбое запроса.
    pub async fn list_by_user_and_course(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        course_id: CourseId,
    ) -> Result<Vec<Attempt>, AttemptRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            FROM attempts
            WHERE user_id = $1 AND course_id = $2 AND tenant_id = $3
            ORDER BY started_at DESC
            "#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(Self::row_to_attempt).collect()
    }

    /// Возвращает все попытки по курсу (для инструктора).
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку базы данных при сбое запроса.
    pub async fn list_by_course(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<Vec<Attempt>, AttemptRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, tenant_id, user_id, course_id, status,
                started_at, completed_at, score, passed,
                time_limit_seconds, time_spent_seconds, attempt_number,
                created_at, updated_at
            FROM attempts
            WHERE course_id = $1 AND tenant_id = $2
            ORDER BY started_at DESC
            "#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(Self::row_to_attempt).collect()
    }

    /// Проверяет, существует ли активная попытка пользователя по курсу.
    ///
    /// Используется для предотвращения одновременных попыток.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку базы данных при сбое запроса.
    pub async fn has_active_attempt(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        course_id: CourseId,
    ) -> Result<bool, AttemptRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt
            FROM attempts
            WHERE user_id = $1 AND course_id = $2 AND tenant_id = $3
              AND status = 'in_progress'
            "#,
        )
        .bind(user_id.0)
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.get("cnt");
        Ok(count > 0)
    }

    /// Преобразует строку базы данных в модель `Attempt`.
    fn row_to_attempt(row: &sqlx::postgres::PgRow) -> Result<Attempt, AttemptRepositoryError> {
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let user_id: Uuid = row.get("user_id");
        let course_id: Uuid = row.get("course_id");
        let status_str: String = row.get("status");
        let started_at: DateTime<Utc> = row.get("started_at");
        let completed_at: Option<DateTime<Utc>> = row.get("completed_at");
        let score: Option<f64> = row.get("score");
        let passed: Option<bool> = row.get("passed");
        let time_limit_seconds: Option<i32> = row.get("time_limit_seconds");
        let time_spent_seconds: i32 = row.get("time_spent_seconds");
        let attempt_number: i32 = row.get("attempt_number");

        // Парсим статус попытки
        let status = Self::parse_attempt_status(&status_str)?;

        Ok(Attempt {
            id: AttemptId(id),
            tenant_id: TenantId(tenant_id),
            user_id: UserId(user_id),
            course_id: CourseId(course_id),
            status,
            started_at,
            completed_at,
            score,
            passed,
            time_limit_seconds,
            time_spent_seconds,
            attempt_number,
        })
    }

    /// Парсит строковое представление статуса попытки в `AttemptStatus`.
    fn parse_attempt_status(s: &str) -> Result<AttemptStatus, AttemptRepositoryError> {
        match s {
            "in_progress" => Ok(AttemptStatus::InProgress),
            "completed" => Ok(AttemptStatus::Completed),
            "timed_out" => Ok(AttemptStatus::TimedOut),
            "abandoned" => Ok(AttemptStatus::Abandoned),
            _ => Err(AttemptRepositoryError::InvalidStatus(s.to_string())),
        }
    }
}