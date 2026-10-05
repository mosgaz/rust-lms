// crates/api/src/database/repositories/user.rs
//! Репозиторий для работы с сущностью User (связь личности с тенантом).
//!
//! User — это не человек, а роль личности в конкретном тенанте.
//! Один и тот же человек (Identity) может иметь несколько записей User
//! в разных тенантах.

use rust_lms_shared::{IdentityId, TenantId, User, UserId};
use sqlx::FromRow;
use sqlx::PgPool;
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
    /// Пользователь с таким identity_id уже существует в тенанте.
    #[error("user with identity_id '{identity_id}' already exists in tenant {tenant_id}")]
    IdentityAlreadyExists {
        /// Идентификатор личности.
        identity_id: IdentityId,
        /// Идентификатор тенанта.
        tenant_id: TenantId,
    },
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления записями User.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Создаёт новый репозиторий User.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новую запись User (связь личности с тенантом).
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError::IdentityAlreadyExists`, если эта личность
    /// уже имеет запись в данном тенанте.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        identity_id: IdentityId,
    ) -> Result<User, UserRepositoryError> {
        let user_id = UserId::new();

        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            identity_id = %identity_id,
            "Creating new user record"
        );

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, tenant_id, identity_id, is_active)
            VALUES ($1, $2, $3, true)
            RETURNING id, tenant_id, identity_id, is_active
            "#,
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .bind(identity_id.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some("23505") {
                    return UserRepositoryError::IdentityAlreadyExists {
                        identity_id,
                        tenant_id,
                    };
                }
            }
            UserRepositoryError::DatabaseError(e)
        })?;

        tx.commit().await?;

        Ok(User {
            id: user_id,
            tenant_id,
            identity_id,
            is_active: row.is_active,
        })
    }

    /// Получает запись User по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError::NotFound`, если запись не найдена.
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

        let row: UserRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, identity_id, is_active
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
            identity_id: IdentityId(row.identity_id),
            is_active: row.is_active,
        })
    }

    /// Получает список активных тенантов для данной личности.
    ///
    /// Используется при логине для определения доступных тенантов.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError`, если не удалось выполнить запрос.
    pub async fn find_active_tenants_for_identity(
        &self,
        identity_id: IdentityId,
    ) -> Result<Vec<TenantId>, UserRepositoryError> {
        tracing::debug!(identity_id = %identity_id, "Fetching active tenants for identity");

        let rows: Vec<TenantIdRow> = sqlx::query_as(
            r#"
            SELECT u.tenant_id
            FROM users u
            JOIN tenants t ON u.tenant_id = t.id
            WHERE u.identity_id = $1 AND u.is_active = true AND t.is_active = true
            "#,
        )
        .bind(identity_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| TenantId(r.tenant_id)).collect())
    }

    /// Проверяет, активна ли запись User для данной личности в указанном тенанте.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError`, если не удалось выполнить запрос.
    pub async fn is_user_active_in_tenant(
        &self,
        identity_id: IdentityId,
        tenant_id: TenantId,
    ) -> Result<bool, UserRepositoryError> {
        tracing::debug!(
            identity_id = %identity_id,
            tenant_id = %tenant_id,
            "Checking if user is active in tenant"
        );

        let row: Option<ActiveRow> = sqlx::query_as(
            r#"
            SELECT is_active
            FROM users
            WHERE identity_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(identity_id.0)
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.is_active).unwrap_or(false))
    }
}

/// Внутренняя структура для маппинга User из БД.
#[derive(Debug, FromRow)]
struct UserRow {
    #[allow(dead_code)]
    id: uuid::Uuid,
    #[allow(dead_code)]
    tenant_id: uuid::Uuid,
    identity_id: uuid::Uuid,
    is_active: bool,
}

/// Внутренняя структура для маппинга tenant_id.
#[derive(Debug, FromRow)]
struct TenantIdRow {
    tenant_id: uuid::Uuid,
}

/// Внутренняя структура для маппинга is_active.
#[derive(Debug, FromRow)]
struct ActiveRow {
    is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let user_id = UserId::new();
        let err = UserRepositoryError::NotFound(user_id);
        let msg = err.to_string();
        assert!(msg.contains("user not found"));
    }

    #[test]
    fn test_error_display_identity_exists() {
        let tenant_id = TenantId::new();
        let identity_id = IdentityId::new();
        let err = UserRepositoryError::IdentityAlreadyExists {
            identity_id,
            tenant_id,
        };
        let msg = err.to_string();
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn test_user_row_mapping() {
        let _row = UserRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            identity_id: uuid::Uuid::new_v4(),
            is_active: true,
        };
    }

    #[test]
    fn test_tenant_id_row_mapping() {
        let _row = TenantIdRow {
            tenant_id: uuid::Uuid::new_v4(),
        };
    }
}