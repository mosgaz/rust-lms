// crates/shared/src/models/certificate.rs
//! Модель сертификата (Certificate).
//!
//! Сертификат выдаётся пользователю при успешном завершении курса (программы, потока).
//! Публичный `verification_hash` используется для проверки подлинности через публичный эндпоинт.
//!
//! DTO вынесен в `shared`, чтобы клиент мог получать его через `#[server]`-функции.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор сертификата.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct CertificateId(pub Uuid);

impl CertificateId {
    /// Генерирует новый случайный идентификатор сертификата (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CertificateId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CertificateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CertificateId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Реестр выданного цифрового сертификата.
///
/// ⚠️ `course_title` **не хранится** в сертификате. Название курса/программы/потока
/// подтягивается отдельно через репозитории соответствующих сущностей.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Certificate {
    /// Уникальный идентификатор сертификата.
    pub id: Uuid,
    /// Идентификатор тенанта (организации), выдавшей сертификат.
    pub tenant_id: Uuid,
    /// Идентификатор пользователя, получившего сертификат.
    pub user_id: Uuid,
    /// Тип сущности, за которую выдан сертификат ('course', 'program', 'batch').
    pub target_type: String,
    /// Идентификатор целевой сущности.
    pub target_id: Uuid,
    /// Уникальный публичный хэш-код для верификации подлинности.
    pub verification_hash: String,
    /// Дата и время выдачи сертификата.
    pub issued_at: DateTime<Utc>,
}