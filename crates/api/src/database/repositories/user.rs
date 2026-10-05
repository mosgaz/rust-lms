// crates/api/src/database/repositories/user.rs
//! Репозиторий для работы с сущностью User и Credentials.
//!
//! Содержит методы для CRUD-операций над пользователями, а также для работы
//! с учётными данными (email + password_hash) при аутентификации.

use rust_lms_shared::{Credentials, TenantId, User, UserId};
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
    /// Пользователь с таким email уже существует в тенанте.
    #[error("user with email '{email}' already exists in tenant {tenant_id}")]
    EmailAlreadyExists {
        /// Электронная почта, которая уже занята.
        email: String,
        /// Идентификатор тенанта, в котором произошла коллизия.
        tenant_id: TenantId,
    },
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
}

/// Репозиторий для управления пользователями.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Создает новый репозиторий User.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт нового пользователя с паролем (хэш Argon2id).
    ///
    /// # Arguments
    ///
    /// * `tenant_id` — идентификатор тенанта для RLS-контекста.
    /// * `email` — электронная почта пользователя (уникальна в рамках тенанта).
    /// * `password_hash` — PHC-хэш пароля в формате `$argon2id$...`.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError::EmailAlreadyExists`, если пользователь
    /// с таким email уже существует в тенанте.
    pub async fn create_with_password(
        &self,
        tenant_id: TenantId,
        email: &str,
        password_hash: &str,
    ) -> Result<User, UserRepositoryError> {
        let user_id = UserId::new();

        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            %email,
            "Creating new user with password"
        );

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash, is_active)
            VALUES ($1, $2, $3, $4, true)
            RETURNING id, tenant_id, email, is_active
            "#,
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            // Обработка нарушения уникальности (tenant_id, email)
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some("23505") {
                    return UserRepositoryError::EmailAlreadyExists {
                        email: email.to_string(),
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
            email: row.email,
            is_active: row.is_active,
        })
    }

    /// Получает учётные данные пользователя по email для аутентификации.
    ///
    /// **Важно:** перед вызовом метода устанавливается RLS-контекст,
    /// поэтому поиск происходит строго в рамках указанного тенанта.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` — идентификатор тенанта для RLS-контекста.
    /// * `email` — электронная почта пользователя.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError::NotFound`, если пользователь не найден.
    pub async fn find_credentials_by_email(
        &self,
        tenant_id: TenantId,
        email: &str,
    ) -> Result<Credentials, UserRepositoryError> {
        tracing::debug!(
            %email,
            tenant_id = %tenant_id,
            "Fetching credentials by email"
        );

        let mut tx = self.pool.begin().await?;

        // Устанавливаем RLS-контекст
        let rls_context = RlsContext::new(tenant_id);
        rls_context.apply(&mut *tx).await?;

        // TODO(migration): перейти на sqlx::query_as! с compile-time проверкой
        // после поднятия PostgreSQL и применения миграций (см. CODING_STANDARDS.md §2.4).
        let row: CredentialsRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, email, password_hash, is_active
            FROM users
            WHERE email = $1 AND is_active = true
            "#,
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| UserRepositoryError::NotFound(UserId::default()))?;

        tx.commit().await?;

        Ok(Credentials {
            user_id: UserId(row.id),
            tenant_id: TenantId(row.tenant_id),
            email: row.email,
            password_hash: row.password_hash,
            is_active: row.is_active,
        })
    }

    /// Получает пользователя по идентификатору.
    ///
    /// # Errors
    ///
    /// Возвращает `UserRepositoryError::NotFound`, если пользователь не найден.
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
    /// Возвращает `UserRepositoryError::NotFound`, если пользователь не найден.
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

/// Внутренняя структура для маппинга результатов запросов (без password_hash).
#[derive(Debug, FromRow)]
struct UserRow {
    id: uuid::Uuid,
    #[allow(dead_code)]
    tenant_id: uuid::Uuid,
    email: String,
    is_active: bool,
}

/// Внутренняя структура для маппинга Credentials из БД.
#[derive(Debug, FromRow)]
struct CredentialsRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    email: String,
    password_hash: String,
    is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit-тесты для маппинга структур (не требуют БД).

    #[test]
    fn test_user_row_mapping() {
        // Проверяем, что структура UserRow корректно определена
        let _row = UserRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
            is_active: true,
        };
    }

    #[test]
    fn test_credentials_row_mapping() {
        // Проверяем, что структура CredentialsRow корректно определена
        let _row = CredentialsRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$placeholder".to_string(),
            is_active: true,
        };
    }

    #[test]
    fn test_error_display_not_found() {
        let user_id = UserId::new();
        let err = UserRepositoryError::NotFound(user_id);
        let msg = err.to_string();
        assert!(msg.contains("user not found"));
    }

    #[test]
    fn test_error_display_email_exists() {
        let tenant_id = TenantId::new();
        let err = UserRepositoryError::EmailAlreadyExists {
            email: "test@example.com".to_string(),
            tenant_id,
        };
        let msg = err.to_string();
        assert!(msg.contains("test@example.com"));
        assert!(msg.contains("already exists"));
    }
}