// crates/api/src/http/handlers/mod.rs
//! REST-обработчики, разделённые по доменам.

use serde::Serialize;

pub mod auth;
pub mod batch;
pub mod batch_enrollment;
pub mod course;
pub mod course_enrollment;
pub mod node;
pub mod tenant;
pub mod user;

pub use auth::{login, refresh, select_tenant};
pub use batch::{create_batch, delete_batch, get_batch, list_batches, update_batch};
pub use batch_enrollment::{
    enroll_to_batch, list_batch_enrollments, unenroll_from_batch, update_batch_enrollment_role,
};
pub use course::{
    create_course, delete_course, get_course, list_courses, publish_course, update_course,
};
pub use course_enrollment::{
    enroll_to_course, list_course_enrollments, list_user_course_enrollments, unenroll_from_course,
};
pub use node::{
    create_child_node, create_root_node, delete_node, get_course_tree, get_node, get_node_subtree,
    move_node, update_node,
};
pub use tenant::{create_tenant, get_tenant};
pub use user::{create_user, get_user};

use crate::auth::AuthService;
use crate::database::{
    BatchEnrollmentRepository, BatchRepository, CourseEnrollmentRepository, CourseRepository,
    NodeRepository, TenantRepository, UserRepository,
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
    /// Репозиторий для работы с потоками (batches).
    pub batch_repo: BatchRepository,
    /// Репозиторий для работы с зачислениями в потоки.
    pub batch_enrollment_repo: BatchEnrollmentRepository,
    /// Репозиторий для работы с индивидуальными зачислениями на курсы.
    pub course_enrollment_repo: CourseEnrollmentRepository,
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