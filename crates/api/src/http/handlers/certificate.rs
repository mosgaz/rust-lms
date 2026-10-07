// crates/api/src/http/handlers/certificate.rs
//! HTTP-обработчики для работы с сертификатами.
//!
//! Содержит эндпоинты для получения списка сертификатов пользователя,
//! публичной верификации и скачивания PDF-файла.

use crate::auth::JwtClaims;
use crate::database::{Certificate, RlsContext};
use crate::http::handlers::AppState;
use axum::{
    body::Body,
    extract::{Extension, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
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
#[axum::debug_handler]
pub async fn list_my_certificates(
    State(state): State<AppState>,
    Extension(ctx): Extension<RlsContext>,
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

/// Скачивает PDF-сертификат по его идентификатору.
///
/// PDF генерируется на лету из метаданных сертификата.
#[axum::debug_handler]
pub async fn download_certificate_pdf(
    State(state): State<AppState>,
    Extension(ctx): Extension<RlsContext>,
    Path(cert_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let (cert, user_name, course_title) = state
        .certificate_service
        .get_certificate_with_details(&ctx, cert_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let pdf_bytes = state
        .certificate_service
        .generate_certificate_pdf(&cert, &user_name, &course_title)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("certificate_{}.pdf", cert.verification_hash);
    
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/pdf"));
    
    let content_disposition = format!("attachment; filename=\"{}\"", filename);
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&content_disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );

    Ok((headers, Body::from(pdf_bytes)))
}