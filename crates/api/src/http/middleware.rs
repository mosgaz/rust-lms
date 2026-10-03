// crates/api/src/http/middleware.rs
//! Middleware для извлечения контекста тенанта из HTTP-запросов.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use rust_lms_shared::TenantId;
use uuid::Uuid;

/// Извлекает `tenant_id` из заголовка `X-Tenant-ID` и добавляет его в расширения запроса.
///
/// # Errors
///
/// Возвращает `StatusCode::BAD_REQUEST`, если заголовок отсутствует или содержит невалидный UUID.
pub async fn extract_tenant_context(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let tenant_id_str = req
        .headers()
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let tenant_uuid = Uuid::parse_str(tenant_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tenant_id = TenantId(tenant_uuid);

    tracing::debug!(tenant_id = %tenant_id, "Extracted tenant context from request");

    req.extensions_mut().insert(tenant_id);

    Ok(next.run(req).await)
}