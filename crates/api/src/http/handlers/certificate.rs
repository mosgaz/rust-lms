// crates/api/src/http/handlers/certificate.rs
//! HTTP-обработчики для работы с сертификатами.
//!
//! Содержит эндпоинты для получения списка сертификатов пользователя
//! и публичной верификации сертификатов по уникальному хешу.

use crate::auth::JwtClaims;
use crate::database::{Certificate, RlsContext};
use crate::http::handlers::AppState;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

/// DTO для ответа API с данными сертификата.
#[derive(Serialize)]
pub struct CertificateResponse {
    /// Уникальный идентификатор сертификата.
    pub id: Uuid,
    /// Тип сущности, за которую выдан сертификат.
    pub target_type: String,
    /// Идентификатор целевой сущности.
    pub target_id: Uuid,
    /// Публичный хэш для верификации.
    pub verification_hash: String,
    /// Дата и время выдачи.
    pub issued_at: chrono::DateTime<chrono::Utc>,
}

impl From<Certificate> for CertificateResponse {
    fn from(cert: Certificate) -> Self {
        Self {
            id: cert.id,
            target_type: cert.target_type,
            target_id: cert.target_id,
            verification_hash: cert.verification_hash,
            issued_at: cert.issued_at,
        }
    }
}

/// Возвращает список всех сертификатов текущего аутентифицированного пользователя.
///
/// Требуется аутентификация через JWT.
#[axum::debug_handler]
pub async fn list_my_certificates(
    State(state): State<AppState>,
    Extension(ctx): Extension<RlsContext>, // <-- ИСПРАВЛЕНО: извлечение через Extension
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Vec<CertificateResponse>>, StatusCode> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let certs = state
        .certificate_service
        .get_user_certificates(&ctx, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(certs.into_iter().map(CertificateResponse::from).collect()))
}

/// Публичная верификация сертификата по хешу.
///
/// Не требует аутентификации, доступна для внешних систем.
#[axum::debug_handler]
pub async fn verify_public(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<CertificateResponse>, StatusCode> {
    let cert = state
        .certificate_service
        .verify_certificate(&hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(CertificateResponse::from(cert)))
}