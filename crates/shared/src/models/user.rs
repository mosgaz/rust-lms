// crates/shared/src/models/user.rs
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::tenant::TenantId;

/// Уникальный идентификатор пользователя.
///
/// Используется newtype-паттерн для типобезопасности и предотвращения
/// перепутывания идентификаторов разных сущностей.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Генерирует новый случайный идентификатор пользователя (UUID v4).
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

// --- Серверные реализации sqlx ---
#[cfg(feature = "server")]
impl sqlx::Type<sqlx::Postgres> for UserId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

#[cfg(feature = "server")]
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for UserId {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let inner = <Uuid as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(UserId(inner))
    }
}

#[cfg(feature = "server")]
impl<'q> sqlx::Encode<'q, sqlx::Postgres> for UserId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <Uuid as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

/// DTO пользователя для обмена данными между слоями.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub tenant_id: TenantId,
    pub email: String,
    pub is_active: bool,
}