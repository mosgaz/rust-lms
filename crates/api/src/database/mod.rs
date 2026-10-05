// crates/api/src/database/mod.rs
//! Управление пулом соединений PostgreSQL и Row-Level Security (RLS).
//!
//! Обеспечивает строгую изоляцию данных между тенантами через сессионные переменные
//! `app.current_tenant_id` (см. ADR 2026.09.28-0001 и CODING_STANDARDS.md §2).

// TODO: раскомментировать при генерации сущностей через sea-orm-cli
// pub mod entities;

pub mod pool;
pub mod repositories;
pub mod rls;

pub use pool::DatabasePool;
pub use repositories::{
    IdentityRepository, IdentityRepositoryError, TenantRepository, TenantRepositoryError,
    UserRepository, UserRepositoryError,
};
pub use rls::RlsContext;