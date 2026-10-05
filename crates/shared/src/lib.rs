// crates/shared/src/lib.rs
//! Общие DTO, модели и контракты платформы rust-lms.
//!
//! Крейт является framework-agnostic: не имеет зависимостей от веб-фреймворков,
//! СУБД-драйверов (кроме `sqlx::Type` для типобезопасности идентификаторов)
//! и UI-библиотек. Используется как общий язык между `api`, `client` и `server`.
//!
//! Архитектура Identity-First:
//! - `Identity` — глобальная сущность (человек, email, пароль).
//! - `User` — связь личности с тенантом (роль в конкретном тенанте).
//!
//! Иерархия контента:
//! - `Course` — основная единица учебного контента.
//! - `Node` — единая сущность для program/course/chapter/topic/lesson.

#![deny(missing_docs)]

// TODO: раскомментировать при наполнении файлов реализацией
// pub mod dto;
pub mod models;
// pub mod xapi;

// --- Flat re-exports для удобства импорта ---

// Базовые идентификаторы и сущности
pub use models::{
    Batch, BatchEnrollment, BatchEnrollmentId, BatchId, BatchRole, BatchStatus, Course, CourseEnrollment,
    CourseEnrollmentId, CourseId, EnrollmentStatus, Identity, IdentityCredentials, IdentityId, Node,
    NodeId, NodeType, Tenant, TenantId, User, UserId,
};

// DTO импорта и синхронизации
// pub use dto::{ImportPayload, SyncPackage};

// xAPI контракты
// pub use xapi::{Actor, Context, Object, Result, Statement, Verb};