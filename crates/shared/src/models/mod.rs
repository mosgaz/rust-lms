// crates/shared/src/models/mod.rs
//! Базовые модели предметной области LMS (сущности).

/// Критерии завершения курса.
pub mod completion;
/// Модель курса.
pub mod course;
/// Учётные данные личности.
pub mod credentials;
/// Глобальная личность.
pub mod identity;
/// Прогресс студента по уроку.
pub mod lesson_progress;
/// Узлы иерархии контента.
pub mod node;
/// Модель арендатора (тенанта).
pub mod tenant;
/// Модель связи личности с тенантом.
pub mod user;

pub use completion::{CompletionCriteria, CompletionCriteriaError, CompletionMode, CompletionRule};
pub use course::{Course, CourseId};
pub use credentials::IdentityCredentials;
pub use identity::{Identity, IdentityId};
pub use lesson_progress::{
    CourseProgressSummary, LessonProgress, LessonProgressId, LessonStatus, ProgressResponse,
    ProgressUpdateRequest, ProgressUpdatedEvent,
};
pub use node::{Node, NodeId, NodeType};
pub use tenant::{Tenant, TenantId};
pub use user::{User, UserId};