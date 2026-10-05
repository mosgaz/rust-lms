// crates/shared/src/models/credentials.rs
//! Учётные данные для аутентификации.
//!
//! Содержит данные, необходимые для проверки пароля при логине.
//! Используется только внутри `crates/api/src/auth/` и не экспортируется наружу.

use serde::{Deserialize, Serialize};

use super::identity::IdentityId;
use super::tenant::TenantId;

/// Учётные данные личности для верификации пароля.
///
/// Возвращается `IdentityRepository` при поиске по email.
/// Содержит `password_hash` в PHC-формате (Argon2id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCredentials {
    /// Идентификатор личности.
    pub identity_id: IdentityId,
    /// Электронная почта (для логирования и отладки).
    pub email: String,
    /// Хэш пароля в формате PHC string (`$argon2id$...`).
    pub password_hash: String,
    /// Предпочитаемый тенант (для авто-выбора при логине).
    pub preferred_tenant_id: Option<TenantId>,
}