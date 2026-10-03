// crates/api/src/database/repositories/tenant.rs
//! Репозиторий для работы с сущностью Tenant.

use rust_lms_shared::{Tenant, TenantId};
use sqlx::FromRow;
use sqlx::PgPool;
use thiserror::Error;

/// Ошибки репозитория Tenant.
#[derive(Debug, Error)]
pub enum TenantRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Tenant не найден.
    #[error("tenant not found: {0}")]
    NotFound(TenantId),
}

/// Репозиторий для управления тенантами.
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    /// Создает новый репозиторий Tenant.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создает нового тенанта в базе данных.
    ///
    /// # Errors
    ///
    /// Возвращает `TenantRepositoryError`, если не удалось создать тенанта.
    pub async fn create(&self, slug: &str, name: &str) -> Result<Tenant, TenantRepositoryError> {
        let tenant_id = TenantId::new();

        tracing::info!(tenant_id = %tenant_id, %slug, %name, "Creating new tenant");

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: TenantRow = sqlx::query_as(
            r#"
            INSERT INTO tenants (id, slug, name, is_active)
            VALUES ($1, $2, $3, true)
            RETURNING id, slug, name, is_active
            "#,
        )
        .bind(tenant_id.0)
        .bind(slug)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(Tenant {
            id: tenant_id,
            slug: row.slug,
            name: row.name,
            is_active: row.is_active,
        })
    }

    /// Получает тенанта по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает `TenantRepositoryError`, если тенант не найден или произошла ошибка БД.
    pub async fn find_by_id(&self, tenant_id: TenantId) -> Result<Tenant, TenantRepositoryError> {
        tracing::debug!(tenant_id = %tenant_id, "Fetching tenant by ID");

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: TenantRow = sqlx::query_as(
            r#"
            SELECT id, slug, name, is_active
            FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(TenantRepositoryError::NotFound(tenant_id))?;

        Ok(Tenant {
            id: tenant_id,
            slug: row.slug,
            name: row.name,
            is_active: row.is_active,
        })
    }

    /// Получает тенанта по slug.
    ///
    /// # Errors
    ///
    /// Возвращает `TenantRepositoryError`, если тенант не найден или произошла ошибка БД.
    pub async fn find_by_slug(&self, slug: &str) -> Result<Tenant, TenantRepositoryError> {
        tracing::debug!(%slug, "Fetching tenant by slug");

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: TenantRow = sqlx::query_as(
            r#"
            SELECT id, slug, name, is_active
            FROM tenants
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| TenantRepositoryError::NotFound(TenantId::default()))?;

        Ok(Tenant {
            id: TenantId(row.id),
            slug: row.slug,
            name: row.name,
            is_active: row.is_active,
        })
    }
}

/// Внутренняя структура для маппинга результатов запросов к таблице `tenants`.
#[derive(Debug, FromRow)]
struct TenantRow {
    id: uuid::Uuid,
    slug: String,
    name: String,
    is_active: bool,
}