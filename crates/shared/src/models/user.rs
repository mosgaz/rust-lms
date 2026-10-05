// crates/shared/src/models/user.rs
//! Модель связи личности с тенантом (User).
//!
//! User — это не человек, а **роль личности в конкретном тенанте**.
//! Один и тот же человек (Identity) может иметь несколько записей User
//! в разных тенантах.
//!
//! Архитектура Identity-First описана в ADR 2026.09.28-0001.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::identity::IdentityId;
use super::tenant::TenantId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор записи User (связи личности с тенантом).
///
/// Используется newtype-паттерн для типобезопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct UserId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl UserId {
    /// Генерирует новый случайный идентификатор записи User (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO связи личности с тенантом для обмена данными между слоями.
///
/// Не содержит email или password_hash — эти данные принадлежат `Identity`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Уникальный идентификатор записи User.
    pub id: UserId,
    /// Идентификатор тенанта, в котором действует эта запись.
    pub tenant_id: TenantId,
    /// Идентификатор личности, к которой принадлежит эта запись.
    pub identity_id: IdentityId,
    /// Флаг активности записи в данном тенанте.
    pub is_active: bool,
}