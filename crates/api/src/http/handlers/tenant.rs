// crates/api/src/http/handlers/tenant.rs
//! Обработчики для работы с тенантами.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{Tenant, TenantId};
use uuid::Uuid;

use super::{ApiResponse, AppState};

/// Создаёт нового тенанта (заглушка).
pub async fn create_tenant(
    State(_state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let slug = payload
        .get("slug")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Default Tenant");

    let mock_tenant = Tenant {
        id: TenantId(Uuid::new_v4()),
        slug: slug.to_string(),
        name: name.to_string(),
        is_active: true,
    };

    let response = ApiResponse::ok(mock_tenant);
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Получает тенанта по идентификатору (заглушка).
pub async fn get_tenant(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mock_tenant = Tenant {
        id: TenantId(id),
        slug: "mock-slug".to_string(),
        name: "Mock Tenant".to_string(),
        is_active: true,
    };
    let response = ApiResponse::ok(mock_tenant);
    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    // Пока нет чистой логики — можно добавить тесты, когда появится настоящий репозиторий.
}