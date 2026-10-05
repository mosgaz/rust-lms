// crates/shared/src/models/identity.rs
//! Модель глобальной личности (Identity).
//!
//! Identity — это сущность, представляющая человека в системе независимо от тенантов.
//! Один email соответствует ровно одной Identity, но одна Identity может иметь
//! несколько записей в таблице `users` (по одной на каждый тенант).
//!
//! Архитектура Identity-First описана в ADR 2026.09.28-0001.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::tenant::TenantId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор личности (Identity).
///
/// Используется newtype-паттерн для типобезопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct IdentityId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl IdentityId {
    /// Генерирует новый случайный идентификатор личности (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for IdentityId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IdentityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO глобальной личности для обмена данными между слоями.
///
/// Не содержит `password_hash` — это деталь реализации аутентификации,
/// которая используется только внутри `crates/api/src/auth/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Уникальный идентификатор личности.
    pub id: IdentityId,
    /// Электронная почта (уникальна во всей системе).
    pub email: String,
    /// Предпочитаемый тенант для автоматического выбора при логине.
    /// `None`, если у пользователя ещё нет предпочтений (например, он только что
    /// зарегистрировался по приглашению).
    pub preferred_tenant_id: Option<TenantId>,
}