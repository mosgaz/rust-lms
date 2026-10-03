// crates/api/src/database/rls.rs
//! Row-Level Security (RLS) интерцептор для установки контекста тенанта.
//!
//! Перед выполнением любой бизнес-транзакции необходимо явно передать
//! `tenant_id` в сессию PostgreSQL через `set_config('app.current_tenant_id', ...)`
//! (см. CODING_STANDARDS.md §2.1 и ADR 2026.09.28-0001).

use rust_lms_shared::TenantId;
use sqlx::PgConnection;
use thiserror::Error;

/// Ошибки установки RLS-контекста.
#[derive(Debug, Error)]
pub enum RlsError {
    /// Не удалось установить контекст тенанта.
    #[error("failed to set tenant context: {0}")]
    SetContextFailed(#[from] sqlx::Error),
}

/// Контекст Row-Level Security для текущей сессии.
#[derive(Debug, Clone)]
pub struct RlsContext {
    tenant_id: TenantId,
}

impl RlsContext {
    /// Создает новый RLS-контекст для указанного тенанта.
    #[must_use]
    pub fn new(tenant_id: TenantId) -> Self {
        Self { tenant_id }
    }

    /// Применяет контекст тенанта к текущему соединению PostgreSQL.
    ///
    /// Устанавливает сессионную переменную `app.current_tenant_id`,
    /// которая используется политиками RLS для изоляции данных.
    ///
    /// # Errors
    ///
    /// Возвращает `RlsError`, если не удалось установить контекст.
    pub async fn apply(&self, conn: &mut PgConnection) -> Result<(), RlsError> {
        let tenant_id_str = self.tenant_id.0.to_string();

        tracing::debug!(tenant_id = %tenant_id_str, "Setting RLS context for tenant");

        sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
            .bind(tenant_id_str)
            .execute(&mut *conn)
            .await?;

        Ok(())
    }

    /// Возвращает идентификатор тенанта из контекста.
    #[must_use]
    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}