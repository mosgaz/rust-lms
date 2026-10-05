// crates/api/src/database/repositories/mod.rs
//! Репозитории для работы с сущностями базы данных.
//!
//! Все операции записи выполняются через `sqlx::query_as` с ручным маппингом
//! через derive-макрос `FromRow` (см. CODING_STANDARDS.md §2.4).

pub mod tenant;
pub mod user;

pub use tenant::{TenantRepository, TenantRepositoryError};
pub use user::{UserRepository, UserRepositoryError};