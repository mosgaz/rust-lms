// crates/shared/src/models/mod.rs
//! Базовые модели предметной области LMS (сущности).
//!
//! Все модели имеют строгую привязку к `TenantId` для обеспечения
//! изоляции данных между арендаторами (см. ADR 2026.09.28-0001).

// TODO: раскомментировать при наполнении файлов реализацией
// pub mod batch;
// pub mod certificate;
// pub mod course;
// pub mod program;

/// Модель арендатора (тенанта) и его идентификатор.
pub mod tenant;

/// Модель пользователя и его идентификатор.
pub mod user;

// pub use batch::{Batch, BatchId};
// pub use certificate::{Certificate, CertificateId};
// pub use course::{Course, CourseId};
// pub use program::{Program, ProgramId};

pub use tenant::{Tenant, TenantId};
pub use user::{User, UserId};