// crates/shared/src/lib.rs
//! Общие DTO, модели и контракты платформы rust-lms.
//!
//! Крейт является framework-agnostic: не имеет зависимостей от веб-фреймворков,
//! СУБД-драйверов (кроме `sqlx::Type` для типобезопасности идентификаторов)
//! и UI-библиотек. Используется как общий язык между `api`, `client` и `server`.

#![deny(missing_docs)]

// pub mod dto;
pub mod models;
// pub mod xapi;

// --- Flat re-exports для удобства импорта ---

// Базовые идентификаторы и сущности
pub use models::{
    // Batch, BatchId, 
	// Certificate, CertificateId, 
	// Course, CourseId, 
	// Program, ProgramId, 
	Tenant, TenantId, 
	User, UserId,
};

// DTO импорта и синхронизации
// pub use dto::{ImportPayload, SyncPackage};

// xAPI контракты
// pub use xapi::{Actor, Context, Object, Result, Statement, Verb};