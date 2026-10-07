// crates/api/src/database/repositories/mod.rs
//! Репозитории для доступа к данным с гарантией RLS-изоляции.

pub mod batch;
pub mod batch_enrollment;
pub mod course;
pub mod course_enrollment;
pub mod identity;
pub mod lesson_progress;
pub mod node;
pub mod tenant;
pub mod user;
pub mod question;
pub mod attempt;
pub mod certificate;

pub use batch::{BatchRepository, BatchRepositoryError};
pub use batch_enrollment::{BatchEnrollmentRepository, BatchEnrollmentRepositoryError};
pub use course::{CourseRepository, CourseRepositoryError};
pub use course_enrollment::{CourseEnrollmentRepository, CourseEnrollmentRepositoryError};
pub use identity::{IdentityRepository, IdentityRepositoryError};
pub use lesson_progress::{
    LessonProgressRepository, LessonProgressRepositoryError, LessonProgressUpdateResult,
};
pub use node::{NodeRepository, NodeRepositoryError};
pub use tenant::{TenantRepository, TenantRepositoryError};
pub use user::{UserRepository, UserRepositoryError};
pub use question::{QuestionRepository, QuestionRepositoryError};
pub use attempt::{AttemptRepository, AttemptRepositoryError, ScoredAnswer};
pub use certificate::{Certificate, CertificateRepository};