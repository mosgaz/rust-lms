// crates/api/src/http/mod.rs
//! HTTP-слой API на базе Axum.
//!
//! Содержит middleware для извлечения контекста тенанта и REST-эндпоинты
//! для работы с сущностями платформы.

pub mod handlers;
pub mod middleware;
pub mod router;

pub use router::create_router;