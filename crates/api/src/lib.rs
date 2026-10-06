// crates/api/src/lib.rs
//! Серверное бэкенд-ядро обработки данных для rust-lms.
//!
//! Содержит бизнес-логику, слой взаимодействия с PostgreSQL (с RLS),
//! LRS-аналитику, ETL-конвейеры и другие серверные компоненты.
//!
//! **Важно:** импорт макросов Leptos сюда аппаратно запрещен.

#![deny(missing_docs)]

pub mod auth;
pub mod database;
pub mod http;
pub mod services; 

// TODO: раскомментировать при наполнении реализации
// pub mod content;
// pub mod etl;
// pub mod features;
// pub mod license;
// pub mod lrs;
// pub mod sbom;
// pub mod scim;

// Re-exports для удобства
pub use auth::{JwtClaims, JwtConfig, JwtManager, PasswordHasher};
pub use database::{DatabasePool, RlsContext, TenantRepository, UserRepository, UserRepositoryError};
pub use http::create_router;
pub use services::ProgressService; // <-- ДОБАВЛЕНО