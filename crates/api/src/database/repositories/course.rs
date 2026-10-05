// crates/api/src/database/repositories/course.rs
//! Репозиторий для работы с курсами (Course).
//!
//! Course — основная единица учебного контента. Содержит главы, темы и уроки
//! через таблицу `nodes` (иерархия контента).

use rust_lms_shared::{Course, CourseId, TenantId};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория Course.
#[derive(Debug, Error)]
pub enum CourseRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Курс не найден.
    #[error("course not found: {0}")]
    NotFound(CourseId),
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления курсами.
#[derive(Clone)]
pub struct CourseRepository {
    pool: PgPool,
}

impl CourseRepository {
    /// Создаёт новый репозиторий Course.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новый курс.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        title: &str,
        title_i18n: Option<JsonValue>,
        description: Option<&str>,
        description_i18n: Option<JsonValue>,
        certification_rules: Option<JsonValue>,
    ) -> Result<Course, CourseRepositoryError> {
        let course_id = CourseId::new();

        tracing::info!(course_id = %course_id, tenant_id = %tenant_id, title = %title, "Creating new course");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: CourseRow = sqlx::query_as(
            r#"
            INSERT INTO courses (id, tenant_id, title, title_i18n, description, description_i18n, version, certification_rules)
            VALUES ($1, $2, $3, $4, $5, $6, 1, $7)
            RETURNING id, tenant_id, title, title_i18n, description, description_i18n, version, certification_rules
            "#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .bind(title)
        .bind(&title_i18n)
        .bind(description)
        .bind(&description_i18n)
        .bind(&certification_rules)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(self.row_to_course(row))
    }

    /// Получает курс по идентификатору.
    pub async fn find_by_id(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<Course, CourseRepositoryError> {
        tracing::debug!(course_id = %course_id, tenant_id = %tenant_id, "Fetching course by ID");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: CourseRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, title, title_i18n, description, description_i18n, version, certification_rules
            FROM courses WHERE id = $1
            "#,
        )
        .bind(course_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(CourseRepositoryError::NotFound(course_id))?;

        tx.commit().await?;
        Ok(self.row_to_course(row))
    }

    /// Получает список курсов тенанта.
    pub async fn find_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Course>, CourseRepositoryError> {
        tracing::debug!(tenant_id = %tenant_id, limit = %limit, offset = %offset, "Fetching courses by tenant");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<CourseRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, title, title_i18n, description, description_i18n, version, certification_rules
            FROM courses ORDER BY created_at DESC LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_course(r)).collect())
    }

    /// Обновляет курс.
    pub async fn update(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
        title: Option<&str>,
        title_i18n: Option<JsonValue>,
        description: Option<&str>,
        description_i18n: Option<JsonValue>,
        certification_rules: Option<JsonValue>,
    ) -> Result<Course, CourseRepositoryError> {
        tracing::info!(course_id = %course_id, "Updating course");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: CourseRow = sqlx::query_as(
            r#"
            UPDATE courses SET
                title = COALESCE($1, title),
                title_i18n = COALESCE($2, title_i18n),
                description = COALESCE($3, description),
                description_i18n = COALESCE($4, description_i18n),
                certification_rules = COALESCE($5, certification_rules),
                updated_at = NOW()
            WHERE id = $6
            RETURNING id, tenant_id, title, title_i18n, description, description_i18n, version, certification_rules
            "#,
        )
        .bind(title)
        .bind(&title_i18n)
        .bind(description)
        .bind(&description_i18n)
        .bind(&certification_rules)
        .bind(course_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(CourseRepositoryError::NotFound(course_id))?;

        tx.commit().await?;
        Ok(self.row_to_course(row))
    }

    /// Удаляет курс (каскадно удалит все связанные nodes через FK).
    pub async fn delete(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<(), CourseRepositoryError> {
        tracing::info!(course_id = %course_id, "Deleting course");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let result = sqlx::query("DELETE FROM courses WHERE id = $1")
            .bind(course_id.0)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CourseRepositoryError::NotFound(course_id));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Публикует новую версию курса (инкремент версии).
    pub async fn publish_version(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<i32, CourseRepositoryError> {
        tracing::info!(course_id = %course_id, "Publishing new course version");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let new_version: i32 = sqlx::query_scalar(
            r#"
            UPDATE courses SET version = version + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING version
            "#,
        )
        .bind(course_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(CourseRepositoryError::NotFound(course_id))?;

        tx.commit().await?;
        Ok(new_version)
    }

    /// Вспомогательная функция для преобразования CourseRow в Course.
    fn row_to_course(&self, row: CourseRow) -> Course {
        Course {
            id: CourseId(row.id),
            tenant_id: TenantId(row.tenant_id),
            title: row.title,
            title_i18n: row.title_i18n,
            description: row.description,
            description_i18n: row.description_i18n,
            version: row.version,
            certification_rules: row.certification_rules,
        }
    }
}

/// Внутренняя структура для маппинга Course из БД.
#[derive(Debug, FromRow)]
struct CourseRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    title: String,
    title_i18n: Option<JsonValue>,
    description: Option<String>,
    description_i18n: Option<JsonValue>,
    version: i32,
    certification_rules: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let course_id = CourseId::new();
        let err = CourseRepositoryError::NotFound(course_id);
        assert!(err.to_string().contains("course not found"));
    }

    #[test]
    fn test_course_row_mapping() {
        let _row = CourseRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            title: "Test Course".to_string(),
            title_i18n: None,
            description: Some("Description".to_string()),
            description_i18n: None,
            version: 1,
            certification_rules: None,
        };
    }
}