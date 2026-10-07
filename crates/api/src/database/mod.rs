// crates/api/src/database/mod.rs
//! Управление пулом соединений PostgreSQL и Row-Level Security (RLS).
//!
//! Обеспечивает строгую изоляцию данных между тенантами через сессионные переменные
//! `app.current_tenant_id` (см. ADR 2026.09.28-0001 и CODING_STANDARDS.md §2).

pub mod pool;
pub mod repositories;
pub mod rls;

pub use pool::DatabasePool;
pub use repositories::{
    BatchEnrollmentRepository, BatchEnrollmentRepositoryError, BatchRepository,
    BatchRepositoryError, CourseEnrollmentRepository, CourseEnrollmentRepositoryError,
    CourseRepository, CourseRepositoryError, IdentityRepository, IdentityRepositoryError,
    LessonProgressRepository, LessonProgressRepositoryError, LessonProgressUpdateResult,
    NodeRepository, NodeRepositoryError, TenantRepository, TenantRepositoryError, UserRepository,
    UserRepositoryError,
	QuestionRepository, QuestionRepositoryError,
	AttemptRepository, AttemptRepositoryError, ScoredAnswer,
	Certificate, CertificateRepository
};
pub use rls::RlsContext;