// crates/shared/src/models/mod.rs
//! Базовые модели предметной области LMS (сущности).

/// Потоки (Batches).
pub mod batch;
/// Зачисления в потоки (Batch Enrollments).
pub mod batch_enrollment;
/// Критерии завершения курса.
pub mod completion;
/// Модель курса.
pub mod course;
/// Зачисления на курсы (Course Enrollments).
pub mod course_enrollment;
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
/// Вопросы для тестов.
pub mod question;
/// Попытки прохождения тестов.
pub mod attempt;
/// Ответы на вопросы.
pub mod answer;

pub use batch::{Batch, BatchId, BatchStatus};
pub use batch_enrollment::{BatchEnrollment, BatchEnrollmentId, BatchRole, EnrollmentStatus};
pub use completion::{CompletionCriteria, CompletionCriteriaError, CompletionMode, CompletionRule};
pub use course::{Course, CourseId};
pub use course_enrollment::{CourseEnrollment, CourseEnrollmentId};
pub use credentials::IdentityCredentials;
pub use identity::{Identity, IdentityId};
pub use lesson_progress::{
    CourseProgressSummary, LessonProgress, LessonProgressId, LessonStatus, ProgressResponse,
    ProgressUpdateRequest, ProgressUpdatedEvent,
};
pub use node::{Node, NodeId, NodeType};
pub use tenant::{Tenant, TenantId};
pub use user::{User, UserId};
pub use question::{Question, QuestionId, QuestionType, AnswerOption, CreateQuestionRequest, UpdateQuestionRequest};
pub use attempt::{Attempt, AttemptId, AttemptStatus, CreateAttemptRequest, AttemptResponse, CompleteAttemptRequest};
pub use answer::{Answer, AnswerId, CreateAnswerRequest, AnswerResponse};