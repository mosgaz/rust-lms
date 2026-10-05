// crates/shared/src/models/mod.rs
//! Базовые модели предметной области LMS (сущности).
//!
//! Архитектура Identity-First:
//! - `Identity` — глобальная сущность (человек, email, пароль).
//! - `User` — связь личности с тенантом (роль в конкретном тенанте).
//! - `Tenant` — арендатор (организация).
//!
//! Иерархия контента (Adjacency List + ltree):
//! - `Course` — основная единица учебного контента.
//! - `Node` — единая таблица для program/course/chapter/topic/lesson.
//!
//! Все модели имеют строгую привязку к `TenantId` для обеспечения
//! изоляции данных между арендаторами (см. ADR 2026.09.28-0001).

// TODO: раскомментировать при наполнении файлов реализацией
// pub mod batch;
// pub mod certificate;

/// Модель курса (Course) — основной единицы учебного контента.
pub mod course;

/// Учётные данные личности для аутентификации.
pub mod credentials;

/// Глобальная личность (Identity) — человек в системе.
pub mod identity;

/// Узлы иерархии контента (program/course/chapter/topic/lesson).
pub mod node;

/// Модель арендатора (тенанта) и его идентификатор.
pub mod tenant;

/// Модель связи личности с тенантом (User).
pub mod user;

// pub use batch::{Batch, BatchId};
// pub use certificate::{Certificate, CertificateId};
pub use course::{Course, CourseId};
pub use credentials::IdentityCredentials;
pub use identity::{Identity, IdentityId};
pub use node::{Node, NodeId, NodeType};
pub use tenant::{Tenant, TenantId};
pub use user::{User, UserId};