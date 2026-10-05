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

#![deny(missing_docs)]

// TODO: раскомментировать при наполнении файлов реализацией
// pub mod dto
pub mod models;
// pub mod xapi;

// --- Flat re-exports для удобства импорта ---

// Базовые идентификаторы, сущности и учётные данные (только реализованные)
pub use models::{
    Identity, IdentityCredentials, IdentityId, Tenant, TenantId, User, UserId,
};

// DTO импорта и синхронизации
// pub use dto::{ImportPayload, SyncPackage};

// xAPI контракты
// pub use xapi::{Actor, Context, Object, Result, Statement, Verb};