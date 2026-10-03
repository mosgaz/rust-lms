// crates/shared/src/models/tenant.rs
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;
use uuid::Uuid;

/// Уникальный идентификатор арендатора (тенанта).
///
/// Используется newtype-паттерн для типобезопасности и предотвращения
/// перепутывания идентификаторов разных сущностей.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[sqlx(transparent)]
pub struct TenantId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl TenantId {
    /// Генерирует новый случайный идентификатор тенанта (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO арендатора (тенанта) для обмена данными между слоями.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Уникальный идентификатор тенанта.
    pub id: TenantId,
    /// Уникальный субдомен или ключ тенанта.
    pub slug: String,
    /// Отображаемое название организации.
    pub name: String,
    /// Флаг активности тенанта.
    pub is_active: bool,
}