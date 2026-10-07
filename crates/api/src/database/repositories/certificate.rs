// crates/api/src/database/repositories/certificate.rs
//! Репозиторий для работы с сущностями сертификатов в базе данных.
//!
//! Обеспечивает строгую изоляцию данных через Row-Level Security (RLS)
//! на основе `tenant_id` (см. ADR 2026.09.28-0001).

use crate::database::RlsContext;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Реестр выданного цифрового сертификата.
/// 
/// Защищён Row-Level Security (RLS) по полю `tenant_id`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

/// Репозиторий для работы с сущностями сертификатов в базе данных.
#[derive(Clone)]
pub struct CertificateRepository {
    pool: PgPool,
}

impl CertificateRepository {
    /// Создаёт новый экземпляр репозитория сертификатов.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт и сохраняет новый сертификат в базе данных.
    ///
    /// Генерирует уникальный `verification_hash` для публичной проверки.
    /// Операция выполняется в контексте текущего тенанта (`ctx`).
    pub async fn create(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Certificate> {
        let verification_hash = generate_verification_hash();
        
        let cert = sqlx::query_as::<_, Certificate>(
            r#"
            INSERT INTO certificates (tenant_id, user_id, target_type, target_id, verification_hash, issued_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, tenant_id, user_id, target_type, target_id, verification_hash, issued_at
            "#
        )
        .bind(ctx.tenant_id())
        .bind(user_id)
        .bind(target_type)
        .bind(target_id)
        .bind(&verification_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(cert)
    }

    /// Находит сертификат по его публичному хешу верификации.
    pub async fn find_by_verification_hash(&self, hash: &str) -> Result<Option<Certificate>> {
        Ok(sqlx::query_as::<_, Certificate>(
            r#"
            SELECT id, tenant_id, user_id, target_type, target_id, verification_hash, issued_at
            FROM certificates
            WHERE verification_hash = $1
            "#
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Возвращает список всех сертификатов конкретного пользователя в рамках текущего тенанта.
    pub async fn list_by_user(&self, ctx: &RlsContext, user_id: Uuid) -> Result<Vec<Certificate>> {
        Ok(sqlx::query_as::<_, Certificate>(
            r#"
            SELECT id, tenant_id, user_id, target_type, target_id, verification_hash, issued_at
            FROM certificates
            WHERE tenant_id = $1 AND user_id = $2
            ORDER BY issued_at DESC
            "#
        )
        .bind(ctx.tenant_id())
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?)
    }
}

/// Генерирует криптографически стойкий хэш для верификации сертификата.
fn generate_verification_hash() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(Uuid::new_v4().as_bytes());
    hasher.update(Utc::now().to_rfc3339().as_bytes());
    format!("{:x}", hasher.finalize())
}