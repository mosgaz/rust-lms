// crates/api/src/services/certificate.rs
//! Сервис для управления жизненным циклом сертификатов.
//!
//! Инкапсулирует бизнес-логику выдачи, проверки и получения сертификатов.

use crate::database::{Certificate, CertificateRepository, RlsContext};
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Сервис для управления жизненным циклом сертификатов.
#[derive(Clone)] // <-- ИСПРАВЛЕНО: добавлен derive для совместимости с AppState
pub struct CertificateService {
    repo: CertificateRepository,
}

impl CertificateService {
    /// Создаёт новый экземпляр сервиса сертификатов.
    pub fn new(pool: PgPool) -> Self { // <-- ИСПРАВЛЕНО: PgPool вместо DatabasePool
        Self {
            repo: CertificateRepository::new(pool),
        }
    }

    /// Выдаёт сертификат за успешное завершение курса.
    pub async fn issue_course_certificate(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
        course_id: Uuid,
    ) -> Result<Certificate> {
        self.repo.create(ctx, user_id, "course", course_id).await
    }

    /// Проверяет подлинность сертификата по публичному хешу верификации.
    pub async fn verify_certificate(&self, hash: &str) -> Result<Option<Certificate>> {
        self.repo.find_by_verification_hash(hash).await
    }

    /// Возвращает список всех сертификатов конкретного пользователя.
    pub async fn get_user_certificates(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
    ) -> Result<Vec<Certificate>> {
        self.repo.list_by_user(ctx, user_id).await
    }
}