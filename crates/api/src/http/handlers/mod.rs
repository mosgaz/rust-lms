// crates/api/src/http/handlers/mod.rs
//! REST-обработчики для API, разбитые по сущностям.

pub mod auth;
pub mod course;
pub mod node;
pub mod tenant;
pub mod user;

pub use auth::{login, refresh, select_tenant};
pub use course::{
    create_course, delete_course, get_course, list_courses, publish_course, update_course,
};
pub use node::{
    create_child_node, create_root_node, delete_node, get_course_tree, get_node,
    get_node_subtree, move_node, update_node,
};
pub use tenant::{create_tenant, get_tenant};
pub use user::{create_user, get_user};

use crate::auth::AuthService;
use crate::database::{CourseRepository, NodeRepository, TenantRepository, UserRepository};
use serde::Serialize;

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

impl<T> ApiResponse<T> {
    /// Хелпер для успешного ответа с данными.
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    /// Хелпер для успешного ответа без данных.
    pub fn ok_empty() -> Self {
        Self { success: true, data: None, error: None }
    }

    /// Хелпер для ответа с ошибкой (тип `T` выводится из аннотации).
    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}