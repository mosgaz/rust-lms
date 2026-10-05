// crates/api/src/http/handlers/node.rs
//! Обработчики для работы с узлами иерархии контента.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, NodeId, NodeType, TenantId};
use serde::Deserialize;
use uuid::Uuid;

use crate::database::NodeRepositoryError;

use super::{ApiResponse, AppState};

/// Запрос на создание нового узла иерархии.
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    /// Тип узла (program, course, chapter, topic, lesson).
    pub node_type: NodeType,
    /// Заголовок узла.
    pub title: String,
    /// Локализованные заголовки.
    pub title_i18n: Option<serde_json::Value>,
    /// Описание узла.
    pub description: Option<String>,
    /// Расширяемые метаданные (видео, текст, тест и т.д.).
    pub metadata: serde_json::Value,
}

/// Запрос на обновление узла иерархии.
#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    /// Новый заголовок узла.
    pub title: Option<String>,
    /// Новые локализованные заголовки.
    pub title_i18n: Option<serde_json::Value>,
    /// Новое описание узла.
    pub description: Option<String>,
    /// Новые метаданные.
    pub metadata: Option<serde_json::Value>,
}

/// Запрос на перемещение узла иерархии.
#[derive(Debug, Deserialize)]
pub struct MoveNodeRequest {
    /// Идентификатор нового родительского узла (None для корневых).
    pub new_parent_id: Option<NodeId>,
}

/// Создаёт корневой узел для указанного курса.
pub async fn create_root_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state
        .node_repo
        .create(
            tenant_id,
            None,
            payload.node_type,
            Some(cid),
            &payload.title,
            payload.title_i18n,
            payload.description.as_deref(),
            payload.metadata,
        )
        .await
    {
        Ok(node) => {
            let response = ApiResponse::ok(node);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = match e {
                NodeRepositoryError::CourseNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (status, Json(response)).into_response()
        }
    }
}

/// Создаёт дочерний узел для указанного родителя.
pub async fn create_child_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(parent_id): Path<Uuid>,
    Json(payload): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    let pid = NodeId(parent_id);

    let parent = match state.node_repo.find_by_id(tenant_id, pid).await {
        Ok(p) => p,
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> =
                ApiResponse::err("Parent node not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match state
        .node_repo
        .create(
            tenant_id,
            Some(pid),
            payload.node_type,
            parent.course_id,
            &payload.title,
            payload.title_i18n,
            payload.description.as_deref(),
            payload.metadata,
        )
        .await
    {
        Ok(node) => {
            let response = ApiResponse::ok(node);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает узел по идентификатору.
pub async fn get_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(node_id): Path<Uuid>,
) -> impl IntoResponse {
    let nid = NodeId(node_id);
    match state.node_repo.find_by_id(tenant_id, nid).await {
        Ok(node) => {
            let response = ApiResponse::ok(node);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Node not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает полное дерево узлов для указанного курса.
pub async fn get_course_tree(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.node_repo.find_course_tree(tenant_id, cid).await {
        Ok(nodes) => {
            let response = ApiResponse::ok(nodes);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает поддерево начиная с указанного узла (через ltree).
pub async fn get_node_subtree(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(node_id): Path<Uuid>,
) -> impl IntoResponse {
    let nid = NodeId(node_id);
    match state.node_repo.find_subtree(tenant_id, nid).await {
        Ok(nodes) => {
            let response = ApiResponse::ok(nodes);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Node not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Обновляет метаданные узла (title, description, metadata).
pub async fn update_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(node_id): Path<Uuid>,
    Json(payload): Json<UpdateNodeRequest>,
) -> impl IntoResponse {
    let nid = NodeId(node_id);
    match state
        .node_repo
        .update(
            tenant_id,
            nid,
            payload.title.as_deref(),
            payload.title_i18n,
            payload.description.as_deref(),
            payload.metadata,
        )
        .await
    {
        Ok(node) => {
            let response = ApiResponse::ok(node);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Node not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Перемещает узел под нового родителя (обновляет ltree path).
pub async fn move_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(node_id): Path<Uuid>,
    Json(payload): Json<MoveNodeRequest>,
) -> impl IntoResponse {
    let nid = NodeId(node_id);
    match state
        .node_repo
        .move_node(tenant_id, nid, payload.new_parent_id)
        .await
    {
        Ok(node) => {
            let response = ApiResponse::ok(node);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> =
                ApiResponse::err("Node or new parent not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Удаляет узел и всё его поддерево (каскадно).
pub async fn delete_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(node_id): Path<Uuid>,
) -> impl IntoResponse {
    let nid = NodeId(node_id);
    match state.node_repo.delete(tenant_id, nid).await {
        Ok(()) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::ok_empty();
            (StatusCode::NO_CONTENT, Json(response)).into_response()
        }
        Err(NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Node not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Маппинг NodeRepositoryError -> StatusCode для create_root_node.
    fn create_root_status(err: &NodeRepositoryError) -> StatusCode {
        match err {
            NodeRepositoryError::CourseNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Маппинг NodeRepositoryError -> StatusCode для get/update/move/delete.
    fn node_status(err: &NodeRepositoryError) -> StatusCode {
        match err {
            NodeRepositoryError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[test]
    fn test_course_not_found_maps_to_404() {
        let err = NodeRepositoryError::CourseNotFound(CourseId(uuid::Uuid::nil()));
        assert_eq!(create_root_status(&err), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_node_not_found_maps_to_404() {
        let err = NodeRepositoryError::NotFound(NodeId(uuid::Uuid::nil()));
        assert_eq!(node_status(&err), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_node_request_deserializes_full() {
        let json = serde_json::json!({
            "node_type": "lesson",
            "title": "Lesson 1",
            "description": "Intro",
            "metadata": { "video_url": "https://example.com/v.mp4" },
        });
        let req: CreateNodeRequest = serde_json::from_value(json)
            .expect("CreateNodeRequest must deserialize");
        assert_eq!(req.title, "Lesson 1");
        assert_eq!(req.description.as_deref(), Some("Intro"));
        assert_eq!(req.metadata["video_url"], "https://example.com/v.mp4");
    }

    #[test]
    fn test_update_node_request_all_optional() {
        let json = serde_json::json!({});
        let req: UpdateNodeRequest = serde_json::from_value(json)
            .expect("UpdateNodeRequest must deserialize from empty object");
        assert!(req.title.is_none());
        assert!(req.description.is_none());
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_move_node_request_without_parent_is_root() {
        let json = serde_json::json!({});
        let req: MoveNodeRequest = serde_json::from_value(json)
            .expect("MoveNodeRequest must deserialize");
        assert!(req.new_parent_id.is_none());
    }

    #[test]
    fn test_move_node_request_with_parent() {
        let parent = uuid::Uuid::new_v4();
        let json = serde_json::json!({ "new_parent_id": parent });
        let req: MoveNodeRequest = serde_json::from_value(json)
            .expect("MoveNodeRequest must deserialize");
        assert_eq!(req.new_parent_id.map(|n| n.0), Some(parent));
    }
}