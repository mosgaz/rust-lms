// crates/api/src/database/repositories/course_enrollment.rs
//! Репозиторий для работы с индивидуальными зачислениями на курсы (Course Enrollment).

use chrono::Utc;
use rust_lms_shared::{
    CourseEnrollment, CourseEnrollmentId, CourseId, EnrollmentStatus, UserId,
};
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория CourseEnrollment.
#[derive(Debug, Error)]
pub enum CourseEnrollmentRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Зачисление не найдено.
    #[error("course enrollment not found")]
    NotFound,
    /// Пользователь уже зачислен на курс.
    #[error("user {user_id} is already enrolled in course {course_id}")]
    AlreadyEnrolled {
        /// Идентификатор пользователя.
        user_id: UserId,
        /// Идентификатор курса.
        course_id: CourseId,
    },
    /// Курс не найден.
    #[error("course not found: {0}")]
    CourseNotFound(CourseId),
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления индивидуальными зачислениями на курсы.
#[derive(Clone)]
pub struct CourseEnrollmentRepository {
    pool: PgPool,
}

impl CourseEnrollmentRepository {
    /// Создаёт новый репозиторий CourseEnrollment.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Зачисляет пользователя на курс.
    ///
    /// # Errors
    /// Возвращает `AlreadyEnrolled`, если пользователь уже зачислен.
    pub async fn enroll(
        &self,
        course_id: CourseId,
        user_id: UserId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<CourseEnrollment, CourseEnrollmentRepositoryError> {
        let enrollment_id = CourseEnrollmentId::new();

        tracing::info!(
            enrollment_id = %enrollment_id,
            course_id = %course_id,
            user_id = %user_id,
            "Enrolling user to course"
        );

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        // Проверяем, что курс существует (и принадлежит тенанту через RLS)
        let course_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM courses WHERE id = $1)")
            .bind(course_id.0)
            .fetch_one(&mut *tx)
            .await?;

        if !course_exists {
            return Err(CourseEnrollmentRepositoryError::CourseNotFound(course_id));
        }

        // Проверяем дубликат
        let already_enrolled: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course_enrollments WHERE course_id = $1 AND user_id = $2 AND status != 'dropped')"
        )
        .bind(course_id.0)
        .bind(user_id.0)
        .fetch_one(&mut *tx)
        .await?;

        if already_enrolled {
            return Err(CourseEnrollmentRepositoryError::AlreadyEnrolled { user_id, course_id });
        }

        let row: CourseEnrollmentRow = sqlx::query_as(
            r#"
            INSERT INTO course_enrollments (id, course_id, user_id, enrolled_at, progress, status)
            VALUES ($1, $2, $3, NOW(), 0.0, 'active')
            RETURNING id, course_id, user_id, enrolled_at, completed_at, progress, status
            "#,
        )
        .bind(enrollment_id.0)
        .bind(course_id.0)
        .bind(user_id.0)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(self.row_to_enrollment(row))
    }

    /// Отчисляет пользователя с курса (soft delete: status = dropped).
    pub async fn unenroll(
        &self,
        course_id: CourseId,
        user_id: UserId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<(), CourseEnrollmentRepositoryError> {
        tracing::info!(course_id = %course_id, user_id = %user_id, "Unenrolling user from course");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let result = sqlx::query(
            "UPDATE course_enrollments SET status = 'dropped', completed_at = NOW() WHERE course_id = $1 AND user_id = $2 AND status = 'active'"
        )
        .bind(course_id.0)
        .bind(user_id.0)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CourseEnrollmentRepositoryError::NotFound);
        }

        tx.commit().await?;
        Ok(())
    }

    /// Получает список зачислений на курс.
    pub async fn find_by_course(
        &self,
        course_id: CourseId,
        tenant_id: rust_lms_shared::TenantId,
    ) -> Result<Vec<CourseEnrollment>, CourseEnrollmentRepositoryError> {
        tracing::debug!(course_id = %course_id, "Fetching course enrollments");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<CourseEnrollmentRow> = sqlx::query_as(
            r#"
            SELECT id, course_id, user_id, enrolled_at, completed_at, progress, status
            FROM course_enrollments WHERE course_id = $1 ORDER BY enrolled_at ASC
            "#,
        )
        .bind(course_id.0)
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
    ) -> Result<Vec<CourseEnrollment>, CourseEnrollmentRepositoryError> {
        tracing::debug!(user_id = %user_id, "Fetching user course enrollments");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<CourseEnrollmentRow> = sqlx::query_as(
            r#"
            SELECT id, course_id, user_id, enrolled_at, completed_at, progress, status
            FROM course_enrollments WHERE user_id = $1 ORDER BY enrolled_at DESC
            "#,
        )
        .bind(user_id.0)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_enrollment(r)).collect())
    }

        /// Вспомогательная функция для преобразования CourseEnrollmentRow в CourseEnrollment.
    fn row_to_enrollment(&self, row: CourseEnrollmentRow) -> CourseEnrollment {
        let status = match row.status.as_str() {
            "active" => EnrollmentStatus::Active,
            "completed" => EnrollmentStatus::Completed,
            "dropped" => EnrollmentStatus::Dropped,
            _ => EnrollmentStatus::Active,
        };

        CourseEnrollment {
            id: CourseEnrollmentId(row.id),
            course_id: CourseId(row.course_id),
            user_id: UserId(row.user_id),
            enrolled_at: row.enrolled_at,
            completed_at: row.completed_at,
            progress: row.progress, // <-- Просто присваиваем f64
            status,
        }
    }
}

/// Внутренняя структура для маппинга CourseEnrollment из БД.
#[derive(Debug, FromRow)]
struct CourseEnrollmentRow {
    id: uuid::Uuid,
    course_id: uuid::Uuid,
    user_id: uuid::Uuid,
    enrolled_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
    progress: f64, // <-- Используем f64 вместо Decimal
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_already_enrolled() {
        let err = CourseEnrollmentRepositoryError::AlreadyEnrolled {
            user_id: UserId::new(),
            course_id: CourseId::new(),
        };
        assert!(err.to_string().contains("already enrolled"));
    }

    #[test]
    fn test_enrollment_row_mapping() {
        let row = CourseEnrollmentRow {
            id: uuid::Uuid::new_v4(),
            course_id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            enrolled_at: Utc::now(),
            completed_at: None,
            progress: 0.5,
            status: "active".to_string(),
        };
        let repo = CourseEnrollmentRepository::new(
            PgPool::connect_lazy("postgres://localhost/test").unwrap(),
        );
        let enrollment = repo.row_to_enrollment(row);
        assert!((enrollment.progress - 0.5).abs() < 0.0001);
        assert_eq!(enrollment.status, EnrollmentStatus::Active);
    }
}