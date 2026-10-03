// crates/api/src/database/repositories/user.rs
//! Репозиторий для работы с сущностью User.

use rust_lms_shared::{TenantId, User, UserId};
use sqlx::PgPool;
use sqlx::FromRow;
use thiserror::Error;

use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория User.
#[derive(Debug, Error)]
pub enum UserRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Пользователь не найден.
    #[error("user not found: {0}")]
    NotFound(UserId),
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления пользователями.
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Создает новый репозиторий User.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создает нового пользователя в базе данных.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError`, если не удалось создать пользователя.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        email: &str,
    ) -> Result<User, UserRepositoryError> {
        let user_id = UserId::new();

        tracing::info!(user_id = %user_id, tenant_id = %tenant_id, %email, "Creating new user");

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, tenant_id, email, is_active)
            VALUES ($1, $2, $3, true)
            RETURNING id, tenant_id, email, is_active
            "#,
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .bind(email)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(User {
            id: user_id,
            tenant_id,
            email: row.email,
            is_active: row.is_active,
        })
    }

    /// Получает пользователя по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError`, если пользователь не найден или произошла ошибка БД.
    pub async fn find_by_id(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<User, UserRepositoryError> {
        tracing::debug!(user_id = %user_id, tenant_id = %tenant_id, "Fetching user by ID");

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: UserRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, email, is_active
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(UserRepositoryError::NotFound(user_id))?;

        tx.commit().await?;

        Ok(User {
            id: user_id,
            tenant_id,
            email: row.email,
            is_active: row.is_active,
        })
    }

    /// Получает пользователя по email.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError`, если пользователь не найден или произошла ошибка БД.
    pub async fn find_by_email(
        &self,
        tenant_id: TenantId,
        email: &str,
    ) -> Result<User, UserRepositoryError> {
        tracing::debug!(%email, tenant_id = %tenant_id, "Fetching user by email");

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: UserRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, email, is_active
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| UserRepositoryError::NotFound(UserId::default()))?;

        tx.commit().await?;

        Ok(User {
            id: UserId(row.id),
            tenant_id,
            email: row.email,
            is_active: row.is_active,
        })
    }
}

/// Внутренняя структура для маппинга результатов запросов.
#[derive(Debug, FromRow)]
struct UserRow {
    id: uuid::Uuid,
    #[allow(dead_code)]
    tenant_id: uuid::Uuid,
    email: String,
    is_active: bool,
}