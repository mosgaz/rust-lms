// crates/api/src/database/repositories/batch.rs
//! Репозиторий для работы с потоками (Batch).

use chrono::{DateTime, Utc};
use rust_lms_shared::{Batch, BatchId, BatchStatus, TenantId};
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория Batch.
#[derive(Debug, Error)]
pub enum BatchRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Поток не найден.
    #[error("batch not found: {0}")]
    NotFound(BatchId),
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления потоками.
#[derive(Clone)]
pub struct BatchRepository {
    pool: PgPool,
}

impl BatchRepository {
    /// Создаёт новый репозиторий Batch.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новый поток.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        title: &str,
        description: Option<&str>,
        status: BatchStatus,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        enrollment_deadline: Option<DateTime<Utc>>,
    ) -> Result<Batch, BatchRepositoryError> {
        let batch_id = BatchId::new();

        tracing::info!(
            batch_id = %batch_id,
            tenant_id = %tenant_id,
            title = %title,
            "Creating new batch"
        );

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: BatchRow = sqlx::query_as(
            r#"
            INSERT INTO batches (id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline
            "#,
        )
        .bind(batch_id.0)
        .bind(tenant_id.0)
        .bind(title)
        .bind(description)
        .bind(status.to_string())
        .bind(start_date)
        .bind(end_date)
        .bind(enrollment_deadline)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(self.row_to_batch(row))
    }

    /// Получает поток по идентификатору.
    pub async fn find_by_id(
        &self,
        tenant_id: TenantId,
        batch_id: BatchId,
    ) -> Result<Batch, BatchRepositoryError> {
        tracing::debug!(batch_id = %batch_id, tenant_id = %tenant_id, "Fetching batch by ID");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: BatchRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline
            FROM batches WHERE id = $1
            "#,
        )
        .bind(batch_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(BatchRepositoryError::NotFound(batch_id))?;

        tx.commit().await?;
        Ok(self.row_to_batch(row))
    }

    /// Получает список потоков тенанта с пагинацией.
    pub async fn find_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Batch>, BatchRepositoryError> {
        tracing::debug!(tenant_id = %tenant_id, limit = %limit, offset = %offset, "Fetching batches by tenant");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<BatchRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline
            FROM batches ORDER BY created_at DESC LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_batch(r)).collect())
    }

    /// Получает список активных потоков тенанта.
    pub async fn find_active_batches(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<Batch>, BatchRepositoryError> {
        tracing::debug!(tenant_id = %tenant_id, "Fetching active batches");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<BatchRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline
            FROM batches WHERE status = 'active' ORDER BY start_date DESC
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(rows.into_iter().map(|r| self.row_to_batch(r)).collect())
    }

    /// Обновляет метаданные потока.
    pub async fn update(
        &self,
        tenant_id: TenantId,
        batch_id: BatchId,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<BatchStatus>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        enrollment_deadline: Option<DateTime<Utc>>,
    ) -> Result<Batch, BatchRepositoryError> {
        tracing::info!(batch_id = %batch_id, "Updating batch");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: BatchRow = sqlx::query_as(
            r#"
            UPDATE batches SET
                title = COALESCE($1, title),
                description = COALESCE($2, description),
                status = COALESCE($3, status),
                start_date = COALESCE($4, start_date),
                end_date = COALESCE($5, end_date),
                enrollment_deadline = COALESCE($6, enrollment_deadline),
                updated_at = NOW()
            WHERE id = $7
            RETURNING id, tenant_id, title, description, status, start_date, end_date, enrollment_deadline
            "#,
        )
        .bind(title)
        .bind(description)
        .bind(status.map(|s| s.to_string()))
        .bind(start_date)
        .bind(end_date)
        .bind(enrollment_deadline)
        .bind(batch_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(BatchRepositoryError::NotFound(batch_id))?;

        tx.commit().await?;
        Ok(self.row_to_batch(row))
    }

    /// Удаляет поток (каскадно удалит batch_courses и batch_enrollments).
    pub async fn delete(
        &self,
        tenant_id: TenantId,
        batch_id: BatchId,
    ) -> Result<(), BatchRepositoryError> {
        tracing::info!(batch_id = %batch_id, "Deleting batch");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let result = sqlx::query("DELETE FROM batches WHERE id = $1")
            .bind(batch_id.0)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(BatchRepositoryError::NotFound(batch_id));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Вспомогательная функция для преобразования BatchRow в Batch.
    fn row_to_batch(&self, row: BatchRow) -> Batch {
        let status = match row.status.as_str() {
            "draft" => BatchStatus::Draft,
            "active" => BatchStatus::Active,
            "archived" => BatchStatus::Archived,
            "completed" => BatchStatus::Completed,
            _ => BatchStatus::Draft,
        };

        Batch {
            id: BatchId(row.id),
            tenant_id: TenantId(row.tenant_id),
            title: row.title,
            description: row.description,
            status,
            start_date: row.start_date,
            end_date: row.end_date,
            enrollment_deadline: row.enrollment_deadline,
        }
    }
}

/// Внутренняя структура для маппинга Batch из БД.
#[derive(Debug, FromRow)]
struct BatchRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    title: String,
    description: Option<String>,
    status: String,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    enrollment_deadline: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let batch_id = BatchId::new();
        let err = BatchRepositoryError::NotFound(batch_id);
        assert!(err.to_string().contains("batch not found"));
    }

    #[test]
    fn test_batch_row_mapping() {
        let row = BatchRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            title: "Test Batch".to_string(),
            description: Some("Description".to_string()),
            status: "active".to_string(),
            start_date: None,
            end_date: None,
            enrollment_deadline: None,
        };
        let repo = BatchRepository::new(PgPool::connect_lazy("postgres://localhost/test").unwrap());
        let batch = repo.row_to_batch(row);
        assert_eq!(batch.status, BatchStatus::Active);
    }
}