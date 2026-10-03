// crates/shared/src/models/user.rs
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

use super::tenant::TenantId;

/// Уникальный идентификатор пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[sqlx(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Генерирует новый случайный идентификатор пользователя.
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

/// DTO пользователя для обмена данными между слоями.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Уникальный идентификатор пользователя.
    pub id: UserId,
    /// Идентификатор тенанта, к которому принадлежит пользователь (строгая изоляция).
    pub tenant_id: TenantId,
    /// Электронная почта пользователя.
    pub email: String,
    /// Флаг активности пользователя.
    pub is_active: bool,
}