// crates/shared/src/lib.rs
//! Общие DTO, модели и контракты платформы rust-lms.

#![deny(missing_docs)]

pub mod models;

// --- Flat re-exports для удобства импорта ---

pub use models::{
    Batch, BatchEnrollment, BatchEnrollmentId, BatchId, BatchRole, BatchStatus,
    CompletionCriteria, CompletionCriteriaError, CompletionMode, CompletionRule, Course,
    CourseEnrollment, CourseEnrollmentId, CourseId, CourseProgressSummary, EnrollmentStatus,
    Identity, IdentityCredentials, IdentityId, LessonProgress, LessonProgressId, LessonStatus, Node,
    NodeId, NodeType, ProgressResponse, ProgressUpdateRequest, ProgressUpdatedEvent, Tenant,
    TenantId, User, UserId,
};