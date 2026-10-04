// crates/shared/src/models/tenant.rs
use serde::{Deserialize, Serialize};
use std::fmt;

/// Уникальный идентификатор арендатора (тенанта).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub uuid::Uuid);

impl TenantId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
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

// --- Серверные реализации ---
#[cfg(feature = "server")]
impl sqlx::Type<sqlx::Postgres> for TenantId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <uuid::Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

#[cfg(feature = "server")]
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for TenantId {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let inner = <uuid::Uuid as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(TenantId(inner))
    }
}

#[cfg(feature = "server")]
impl<'q> sqlx::Encode<'q, sqlx::Postgres> for TenantId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <uuid::Uuid as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

/// DTO арендатора (тенанта) для обмена данными между слоями.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub slug: String,
    pub name: String,
    pub is_active: bool,
}