// crates/api/src/database/repositories/batch_enrollment.rs
//! Репозиторий для работы с зачислениями в потоки (Batch Enrollment).

use chrono::Utc;
use rust_lms_shared::{
    BatchEnrollment, BatchEnrollmentId, BatchId, BatchRole, EnrollmentStatus, UserId,
};
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use super::batch::BatchRepositoryError;
use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория BatchEnrollment.
#[derive(Debug, Error)]
pub enum BatchEnrollmentRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Зачисление не найдено.
    #[error("batch enrollment not found: {0}")]
    NotFound(BatchEnrollmentId),
    /// Пользователь уже зачислен в поток.
    #[error("user {user_id} is already enrolled in batch {batch_id}")]
    AlreadyEnrolled {
        /// Идентификатор пользователя.
        user_id: UserId,
        /// Идентификатор потока.
        batch_id: BatchId,
    },
    /// Поток не найден (пробрасывается из BatchRepository).
    #[error("batch not found: {0}")]
    BatchNotFound(BatchId),
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

impl From<BatchRepositoryError> for BatchEnrollmentRepositoryError {
    fn from(err: BatchRepositoryError) -> Self {
        match err {
            BatchRepositoryError::NotFound(id) => Self::BatchNotFound(id),
            BatchRepositoryError::DatabaseError(e) => Self::DatabaseError(e),
            BatchRepositoryError::RlsError(e) => Self::RlsError(e),
        }
    }
}

/// Репозиторий для управления зачислениями в потоки.
#[derive(Clone)]
pub struct BatchEnrollmentRepository {
    pool: PgPool,
}

impl BatchEnrollmentRepository {
    /// Создаёт новый репозиторий BatchEnrollment.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Зачисляет пользователя в поток.
    ///
    /// # Errors
    /// Возвращает `AlreadyEnrolled`, если пользователь уже зачислен.
    pub async fn enroll(
        &self,
        batch_id: BatchId,
        user_id: UserId,
        role: BatchRole,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<BatchEnrollment, BatchEnrollmentRepositoryError> {
        let enrollment_id = BatchEnrollmentId::new();

        tracing::info!(
            enrollment_id = %enrollment_id,
            batch_id = %batch_id,
            user_id = %user_id,
            role = %role,
            "Enrolling user to batch"
        );

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        // Проверяем, что поток существует (и принадлежит тенанту через RLS)
        let batch_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM batches WHERE id = $1)")
            .bind(batch_id.0)
            .fetch_one(&mut *tx)
            .await?;

        if !batch_exists {
            return Err(BatchEnrollmentRepositoryError::BatchNotFound(batch_id));
        }

        // Проверяем дубликат
        let already_enrolled: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM batch_enrollments WHERE batch_id = $1 AND user_id = $2 AND status != 'dropped')"
        )
        .bind(batch_id.0)
        .bind(user_id.0)
        .fetch_one(&mut *tx)
        .await?;

        if already_enrolled {
            return Err(BatchEnrollmentRepositoryError::AlreadyEnrolled { user_id, batch_id });
        }

        let row: BatchEnrollmentRow = sqlx::query_as(
            r#"
            INSERT INTO batch_enrollments (id, batch_id, user_id, role, enrolled_at, status)
            VALUES ($1, $2, $3, $4, NOW(), 'active')
            RETURNING id, batch_id, user_id, role, enrolled_at, completed_at, status
            "#,
        )
        .bind(enrollment_id.0)
        .bind(batch_id.0)
        .bind(user_id.0)
        .bind(role.to_string())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(self.row_to_enrollment(row))
    }

    /// Отчисляет пользователя из потока (soft delete: status = dropped).
    pub async fn unenroll(
        &self,
        batch_id: BatchId,
        user_id: UserId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<(), BatchEnrollmentRepositoryError> {
        tracing::info!(batch_id = %batch_id, user_id = %user_id, "Unenrolling user from batch");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let result = sqlx::query(
            "UPDATE batch_enrollments SET status = 'dropped', completed_at = NOW() WHERE batch_id = $1 AND user_id = $2 AND status = 'active'"
        )
        .bind(batch_id.0)
        .bind(user_id.0)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(BatchEnrollmentRepositoryError::NotFound(BatchEnrollmentId(
                uuid::Uuid::nil(),
            )));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Получает список зачислений в поток.
    pub async fn find_by_batch(
        &self,
        batch_id: BatchId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<Vec<BatchEnrollment>, BatchEnrollmentRepositoryError> {
        tracing::debug!(batch_id = %batch_id, "Fetching batch enrollments");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<BatchEnrollmentRow> = sqlx::query_as(
            r#"
            SELECT id, batch_id, user_id, role, enrolled_at, completed_at, status
            FROM batch_enrollments WHERE batch_id = $1 ORDER BY enrolled_at ASC
            "#,
        )
        .bind(batch_id.0)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_enrollment(r)).collect())
    }

    /// Получает список зачислений пользователя.
    pub async fn find_by_user(
        &self,
        user_id: UserId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<Vec<BatchEnrollment>, BatchEnrollmentRepositoryError> {
        tracing::debug!(user_id = %user_id, "Fetching user batch enrollments");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<BatchEnrollmentRow> = sqlx::query_as(
            r#"
            SELECT id, batch_id, user_id, role, enrolled_at, completed_at, status
            FROM batch_enrollments WHERE user_id = $1 ORDER BY enrolled_at DESC
            "#,
        )
        .bind(user_id.0)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_enrollment(r)).collect())
    }

    /// Обновляет роль участника потока.
    pub async fn update_role(
        &self,
        batch_id: BatchId,
        user_id: UserId,
        new_role: BatchRole,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<BatchEnrollment, BatchEnrollmentRepositoryError> {
        tracing::info!(batch_id = %batch_id, user_id = %user_id, new_role = %new_role, "Updating batch enrollment role");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: BatchEnrollmentRow = sqlx::query_as(
            r#"
            UPDATE batch_enrollments SET role = $1
            WHERE batch_id = $2 AND user_id = $3 AND status != 'dropped'
            RETURNING id, batch_id, user_id, role, enrolled_at, completed_at, status
            "#,
        )
        .bind(new_role.to_string())
        .bind(batch_id.0)
        .bind(user_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(BatchEnrollmentRepositoryError::NotFound(BatchEnrollmentId(
            uuid::Uuid::nil(),
        )))?;

        tx.commit().await?;
        Ok(self.row_to_enrollment(row))
    }

    /// Вспомогательная функция для преобразования BatchEnrollmentRow в BatchEnrollment.
    fn row_to_enrollment(&self, row: BatchEnrollmentRow) -> BatchEnrollment {
        let role = match row.role.as_str() {
            "student" => BatchRole::Student,
            "instructor" => BatchRole::Instructor,
            "tutor" => BatchRole::Tutor,
            "observer" => BatchRole::Observer,
            _ => BatchRole::Student,
        };

        let status = match row.status.as_str() {
            "active" => EnrollmentStatus::Active,
            "completed" => EnrollmentStatus::Completed,
            "dropped" => EnrollmentStatus::Dropped,
            _ => EnrollmentStatus::Active,
        };

        BatchEnrollment {
            id: BatchEnrollmentId(row.id),
            batch_id: BatchId(row.batch_id),
            user_id: UserId(row.user_id),
            role,
            enrolled_at: row.enrolled_at,
            completed_at: row.completed_at,
            status,
        }
    }
}

/// Внутренняя структура для маппинга BatchEnrollment из БД.
#[derive(Debug, FromRow)]
struct BatchEnrollmentRow {
    id: uuid::Uuid,
    batch_id: uuid::Uuid,
    user_id: uuid::Uuid,
    role: String,
    enrolled_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_already_enrolled() {
        let err = BatchEnrollmentRepositoryError::AlreadyEnrolled {
            user_id: UserId::new(),
            batch_id: BatchId::new(),
        };
        assert!(err.to_string().contains("already enrolled"));
    }

    #[test]
    fn test_enrollment_row_mapping() {
        let row = BatchEnrollmentRow {
            id: uuid::Uuid::new_v4(),
            batch_id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            role: "instructor".to_string(),
            enrolled_at: Utc::now(),
            completed_at: None,
            status: "active".to_string(),
        };
        let repo = BatchEnrollmentRepository::new(
            PgPool::connect_lazy("postgres://localhost/test").unwrap(),
        );
        let enrollment = repo.row_to_enrollment(row);
        assert_eq!(enrollment.role, BatchRole::Instructor);
        assert_eq!(enrollment.status, EnrollmentStatus::Active);
    }
}