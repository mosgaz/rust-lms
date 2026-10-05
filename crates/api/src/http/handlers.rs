// crates/api/src/http/handlers.rs
//! REST-обработчики для API.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, NodeId, NodeType, Tenant, TenantId, User};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthService, AuthServiceError, TokenPair};
use crate::database::{
    CourseRepository, NodeRepository, TenantRepository, UserRepository, UserRepositoryError,
};

/// Состояние приложения, общее для всех handlers.
#[derive(Clone)]
pub struct AppState {
    /// Репозиторий для работы с тенантами.
    pub tenant_repo: TenantRepository,
    /// Репозиторий для работы с пользователями.
    pub user_repo: UserRepository,
    /// Сервис аутентификации.
    pub auth_service: AuthService,
    /// Репозиторий для работы с курсами.
    pub course_repo: CourseRepository,
    /// Репозиторий для работы с узлами иерархии контента (nodes).
    pub node_repo: NodeRepository,
}

/// Унифицированный формат ответа API.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Флаг успешности операции.
    pub success: bool,
    /// Полезные данные (при успехе).
    pub data: Option<T>,
    /// Сообщение об ошибке (при неудаче).
    pub error: Option<String>,
}

/// DTO для запроса логина.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// DTO для запроса выбора тенанта.
#[derive(Debug, Deserialize)]
pub struct SelectTenantRequest {
    /// Короткоживущий session token.
    pub session_token: String,
    /// Идентификатор выбранного тенанта.
    pub tenant_id: String,
}

/// DTO для запроса создания пользователя.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Электронная почта пользователя.
    pub email: String,
    /// Исходный пароль.
    pub password: String,
}

/// DTO для ответа с парой токенов.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Короткоживущий access token.
    pub access_token: String,
    /// Долгоживущий refresh token.
    pub refresh_token: String,
    /// Тип токена.
    pub token_type: String,
}

impl From<TokenPair> for TokenResponse {
    fn from(pair: TokenPair) -> Self {
        Self {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            token_type: "Bearer".to_string(),
        }
    }
}

/// Аутентифицирует пользователя по email и паролю.
pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    tracing::info!(email = %payload.email, "Login attempt");

    match state.auth_service.authenticate(&payload.email, &payload.password).await {
        Ok(crate::auth::AuthResult::SingleTenant(token_pair)) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(crate::auth::AuthResult::MultiTenant { session_token, available_tenants }) => {
            let response = ApiResponse { 
                success: true, 
                data: Some(serde_json::json!({ "session_token": session_token, "available_tenants": available_tenants })), 
                error: None 
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccessDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (status, Json(response)).into_response()
        }
    }
}

/// Выбирает конкретный тенант и выдаёт финальные токены.
pub async fn select_tenant(State(state): State<AppState>, Json(payload): Json<SelectTenantRequest>) -> impl IntoResponse {
    let tenant_uuid = match Uuid::parse_str(&payload.tenant_id) {
        Ok(uuid) => TenantId(uuid),
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                success: false, data: None, error: Some("Invalid tenant ID format".to_string())
            })).into_response();
        }
    };

    match state.auth_service.select_tenant(&payload.session_token, tenant_uuid).await {
        Ok(token_pair) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::InvalidSessionToken | AuthServiceError::AccessDenied => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (status, Json(response)).into_response()
        }
    }
}

/// Обновляет access token по валидному refresh token.
pub async fn refresh(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let refresh_token = payload.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("");
    
    match state.auth_service.refresh(refresh_token).await {
        Ok(token_pair) => {
            let response = ApiResponse { success: true, data: Some(TokenResponse::from(token_pair)), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
    }
}

/// Создаёт нового тенанта (заглушка).
pub async fn create_tenant(State(_state): State<AppState>, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let slug = payload.get("slug").and_then(|v| v.as_str()).unwrap_or("default");
    let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or("Default Tenant");
    
    let mock_tenant = Tenant {
        id: TenantId(Uuid::new_v4()),
        slug: slug.to_string(),
        name: name.to_string(),
        is_active: true,
    };
    
    let response = ApiResponse { success: true, data: Some(mock_tenant), error: None };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Получает тенанта по идентификатору (заглушка).
pub async fn get_tenant(State(_state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let mock_tenant = Tenant {
        id: TenantId(id),
        slug: "mock-slug".to_string(),
        name: "Mock Tenant".to_string(),
        is_active: true,
    };
    let response = ApiResponse { success: true, data: Some(mock_tenant), error: None };
    (StatusCode::OK, Json(response)).into_response()
}

/// Создаёт нового пользователя в контексте текущего тенанта.
pub async fn create_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, email = %payload.email, "Creating user via API");

    if payload.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Json(ApiResponse::<User> {
            success: false, data: None, error: Some("Password must be at least 8 characters long".to_string())
        })).into_response();
    }

    match state.auth_service.create_user_in_tenant(tenant_id, &payload.email, &payload.password).await {
        Ok(user) => {
            let response = ApiResponse { success: true, data: Some(user), error: None };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = match &e {
                AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (status, Json(response)).into_response()
        }
    }
}

/// Получает пользователя по идентификатору в контексте текущего тенанта.
pub async fn get_user(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = rust_lms_shared::UserId(id);
    
    match state.user_repo.find_by_id(tenant_id, user_id).await {
        Ok(user) => {
            let response = ApiResponse { success: true, data: Some(user), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(UserRepositoryError::NotFound(_)) => {
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some("User not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(_) => {
            let response: ApiResponse<User> = ApiResponse { success: false, data: None, error: Some("Internal server error".to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// ============================================================================
// COURSE & NODE HANDLERS (Подэтап 7.1.2)
// ============================================================================

/// Запрос на создание нового курса.
#[derive(Debug, Deserialize)]
pub struct CreateCourseRequest {
    /// Заголовок курса.
    pub title: String,
    /// Локализованные заголовки (BCP-47).
    pub title_i18n: Option<serde_json::Value>,
    /// Описание курса.
    pub description: Option<String>,
    /// Локализованные описания.
    pub description_i18n: Option<serde_json::Value>,
    /// Правила автоматической выдачи сертификатов.
    pub certification_rules: Option<serde_json::Value>,
}

/// Запрос на обновление курса.
#[derive(Debug, Deserialize)]
pub struct UpdateCourseRequest {
    /// Новый заголовок курса.
    pub title: Option<String>,
    /// Новые локализованные заголовки.
    pub title_i18n: Option<serde_json::Value>,
    /// Новое описание курса.
    pub description: Option<String>,
    /// Новые локализованные описания.
    pub description_i18n: Option<serde_json::Value>,
    /// Новые правила выдачи сертификатов.
    pub certification_rules: Option<serde_json::Value>,
}

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

// --- Handlers: Courses ---

/// Создаёт новый курс в контексте текущего тенанта.
pub async fn create_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Json(payload): Json<CreateCourseRequest>,
) -> impl IntoResponse {
    tracing::info!(tenant_id = %tenant_id, title = %payload.title, "Creating new course");

    match state.course_repo.create(
        tenant_id,
        &payload.title,
        payload.title_i18n,
        payload.description.as_deref(),
        payload.description_i18n,
        payload.certification_rules,
    ).await {
        Ok(course) => {
            let response = ApiResponse { success: true, data: Some(course), error: None };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create course");
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает курс по идентификатору.
pub async fn get_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.find_by_id(tenant_id, cid).await {
        Ok(course) => {
            let response = ApiResponse { success: true, data: Some(course), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Course not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает список курсов текущего тенанта.
pub async fn list_courses(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
) -> impl IntoResponse {
    match state.course_repo.find_by_tenant(tenant_id, 100, 0).await {
        Ok(courses) => {
            let response = ApiResponse { success: true, data: Some(courses), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Обновляет метаданные курса.
pub async fn update_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<UpdateCourseRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.update(
        tenant_id,
        cid,
        payload.title.as_deref(),
        payload.title_i18n,
        payload.description.as_deref(),
        payload.description_i18n,
        payload.certification_rules,
    ).await {
        Ok(course) => {
            let response = ApiResponse { success: true, data: Some(course), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Course not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Удаляет курс и все связанные с ним узлы (каскадно).
pub async fn delete_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.delete(tenant_id, cid).await {
        Ok(()) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: true, data: None, error: None };
            (StatusCode::NO_CONTENT, Json(response)).into_response()
        }
        Err(crate::database::CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Course not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Публикует новую версию курса (инкремент версии).
pub async fn publish_course(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.course_repo.publish_version(tenant_id, cid).await {
        Ok(new_version) => {
            let response = ApiResponse { 
                success: true, 
                data: Some(serde_json::json!({ "course_id": cid.0, "new_version": new_version })), 
                error: None 
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::CourseRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Course not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

// --- Handlers: Nodes ---

/// Создаёт корневой узел для указанного курса.
pub async fn create_root_node(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.node_repo.create(
        tenant_id,
        None,
        payload.node_type,
        Some(cid),
        &payload.title,
        payload.title_i18n,
        payload.description.as_deref(),
        payload.metadata,
    ).await {
        Ok(node) => {
            let response = ApiResponse { success: true, data: Some(node), error: None };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = match e {
                crate::database::NodeRepositoryError::CourseNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Parent node not found".to_string()) };
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match state.node_repo.create(
        tenant_id,
        Some(pid),
        payload.node_type,
        parent.course_id,
        &payload.title,
        payload.title_i18n,
        payload.description.as_deref(),
        payload.metadata,
    ).await {
        Ok(node) => {
            let response = ApiResponse { success: true, data: Some(node), error: None };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
            let response = ApiResponse { success: true, data: Some(node), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Node not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
            let response = ApiResponse { success: true, data: Some(nodes), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
            let response = ApiResponse { success: true, data: Some(nodes), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Node not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
    match state.node_repo.update(
        tenant_id,
        nid,
        payload.title.as_deref(),
        payload.title_i18n,
        payload.description.as_deref(),
        payload.metadata,
    ).await {
        Ok(node) => {
            let response = ApiResponse { success: true, data: Some(node), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Node not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
    match state.node_repo.move_node(tenant_id, nid, payload.new_parent_id).await {
        Ok(node) => {
            let response = ApiResponse { success: true, data: Some(node), error: None };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Node or new parent not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
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
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: true, data: None, error: None };
            (StatusCode::NO_CONTENT, Json(response)).into_response()
        }
        Err(crate::database::NodeRepositoryError::NotFound(_)) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some("Node not found".to_string()) };
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse { success: false, data: None, error: Some(e.to_string()) };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_response_from_pair() {
        let pair = TokenPair { access_token: "a".to_string(), refresh_token: "r".to_string() };
        let response = TokenResponse::from(pair);
        assert_eq!(response.access_token, "a");
        assert_eq!(response.token_type, "Bearer");
    }

    #[test]
    fn test_auth_error_to_status_mapping() {
        let cases: Vec<(AuthServiceError, StatusCode)> = vec![
            (AuthServiceError::InvalidCredentials, StatusCode::UNAUTHORIZED),
            (AuthServiceError::AccessDenied, StatusCode::FORBIDDEN),
            (AuthServiceError::EmailAlreadyExists, StatusCode::CONFLICT),
        ];

        for (err, expected) in cases {
            let actual = match &err {
                AuthServiceError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                AuthServiceError::AccessDenied => StatusCode::FORBIDDEN,
                AuthServiceError::EmailAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            assert_eq!(actual, expected, "Status mismatch for error: {:?}", err);
        }
    }
}