// crates/shared/src/models/mod.rs
//! Базовые модели предметной области LMS (сущности).
//!
//! Архитектура Identity-First:
//! - `Identity` — глобальная сущность (человек, email, пароль).
//! - `User` — связь личности с тенантом (роль в конкретном тенанте).
//! - `Tenant` — арендатор (организация).
//!
//! Все модели имеют строгую привязку к `TenantId` для обеспечения
//! изоляции данных между арендаторами (см. ADR 2026.09.28-0001).

// TODO: раскомментировать при наполнении файлов реализацией
// pub mod batch;
// pub mod certificate;
// pub mod course;
// pub mod program;

/// Учётные данные личности для аутентификации.
pub mod credentials;

/// Глобальная личность (Identity) — человек в системе.
pub mod identity;

/// Модель арендатора (тенанта) и его идентификатор.
pub mod tenant;

/// Модель связи личности с тенантом (User).
pub mod user;

// pub use batch::{Batch, BatchId};
// pub use certificate::{Certificate, CertificateId};
// pub use course::{Course, CourseId};
// pub use program::{Program, ProgramId};

pub use credentials::IdentityCredentials;
pub use identity::{Identity, IdentityId};
pub use tenant::{Tenant, TenantId};
pub use user::{User, UserId};