// crates/api/src/database/repositories/identity.rs
//! Репозиторий для работы с глобальной сущностью Identity.
//!
//! Identity — это человек в системе (email, password_hash, preferred_tenant_id).
//! Таблица `identities` не имеет RLS, так как это глобальная сущность.

use rust_lms_shared::{Identity, IdentityCredentials, IdentityId, TenantId};
use sqlx::FromRow;
use sqlx::PgPool;
use thiserror::Error;

/// Ошибки репозитория Identity.
#[derive(Debug, Error)]
pub enum IdentityRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Identity не найдена.
    #[error("identity not found: {0}")]
    NotFound(IdentityId),
    /// Email уже зарегистрирован.
    #[error("email '{0}' already registered")]
    EmailAlreadyExists(String),
}

/// Репозиторий для управления сущностями Identity.
#[derive(Clone)]
pub struct IdentityRepository {
    pool: PgPool,
}

impl IdentityRepository {
    /// Создаёт новый репозиторий Identity.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Получает учётные данные Identity по email для аутентификации.
    ///
    /// # Errors
    ///
    /// Возвращает `IdentityRepositoryError::NotFound`, если email не найден.
    pub async fn find_credentials_by_email(
        &self,
        email: &str,
    ) -> Result<IdentityCredentials, IdentityRepositoryError> {
        tracing::debug!(%email, "Fetching identity credentials by email");

        let row: IdentityCredentialsRow = sqlx::query_as(
            r#"
            SELECT id, email, password_hash, preferred_tenant_id
            FROM identities
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| IdentityRepositoryError::NotFound(IdentityId::default()))?;

        Ok(IdentityCredentials {
            identity_id: IdentityId(row.id),
            email: row.email,
            password_hash: row.password_hash,
            preferred_tenant_id: row.preferred_tenant_id.map(TenantId),
        })
    }

    /// Получает Identity по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает `IdentityRepositoryError::NotFound`, если Identity не найдена.
    pub async fn find_by_id(
        &self,
        identity_id: IdentityId,
    ) -> Result<Identity, IdentityRepositoryError> {
        tracing::debug!(identity_id = %identity_id, "Fetching identity by ID");

        let row: IdentityRow = sqlx::query_as(
            r#"
            SELECT id, email, preferred_tenant_id
            FROM identities
            WHERE id = $1
            "#,
        )
        .bind(identity_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(IdentityRepositoryError::NotFound(identity_id))?;

        Ok(Identity {
            id: identity_id,
            email: row.email,
            preferred_tenant_id: row.preferred_tenant_id.map(TenantId),
        })
    }

    /// Обновляет preferred_tenant_id для Identity.
    ///
    /// # Errors
    ///
    /// Возвращает `IdentityRepositoryError`, если не удалось обновить.
    pub async fn update_preferred_tenant(
        &self,
        identity_id: IdentityId,
        tenant_id: Option<TenantId>,
    ) -> Result<(), IdentityRepositoryError> {
        tracing::info!(
            identity_id = %identity_id,
            tenant_id = ?tenant_id,
            "Updating preferred tenant for identity"
        );

        sqlx::query(
            r#"
            UPDATE identities
            SET preferred_tenant_id = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(tenant_id.map(|t| t.0))
        .bind(identity_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Создаёт новую Identity с паролем.
    ///
    /// # Errors
    ///
    /// Возвращает `IdentityRepositoryError::EmailAlreadyExists`, если email уже зарегистрирован.
    pub async fn create_with_password(
        &self,
        email: &str,
        password_hash: &str,
    ) -> Result<IdentityId, IdentityRepositoryError> {
        let identity_id = IdentityId::new();

        tracing::info!(identity_id = %identity_id, %email, "Creating new identity");

        sqlx::query(
            r#"
            INSERT INTO identities (id, email, password_hash)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(identity_id.0)
        .bind(email)
        .bind(password_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some("23505") {
                    return IdentityRepositoryError::EmailAlreadyExists(email.to_string());
                }
            }
            IdentityRepositoryError::DatabaseError(e)
        })?;

        Ok(identity_id)
    }
}

/// Внутренняя структура для маппинга Identity из БД.
#[derive(Debug, FromRow)]
struct IdentityRow {
    #[allow(dead_code)]
    id: uuid::Uuid,
    email: String,
    preferred_tenant_id: Option<uuid::Uuid>,
}

/// Внутренняя структура для маппинга IdentityCredentials из БД.
#[derive(Debug, FromRow)]
struct IdentityCredentialsRow {
    id: uuid::Uuid,
    email: String,
    password_hash: String,
    preferred_tenant_id: Option<uuid::Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let identity_id = IdentityId::new();
        let err = IdentityRepositoryError::NotFound(identity_id);
        let msg = err.to_string();
        assert!(msg.contains("identity not found"));
    }

    #[test]
    fn test_error_display_email_exists() {
        let err = IdentityRepositoryError::EmailAlreadyExists("test@example.com".to_string());
        let msg = err.to_string();
        assert!(msg.contains("test@example.com"));
        assert!(msg.contains("already registered"));
    }

    #[test]
    fn test_identity_row_mapping() {
        let _row = IdentityRow {
            id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
            preferred_tenant_id: Some(uuid::Uuid::new_v4()),
        };
    }

    #[test]
    fn test_identity_credentials_row_mapping() {
        let _row = IdentityCredentialsRow {
            id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$placeholder".to_string(),
            preferred_tenant_id: None,
        };
    }
}