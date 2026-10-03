// crates/api/src/database/repositories/mod.rs
//! Репозитории для работы с сущностями базы данных.
//!
//! Все операции записи выполняются через `sqlx::query!` с compile-time проверкой.
//! Использование `SeaORM` для записи запрещено (см. CODING_STANDARDS.md §2.3).

pub mod tenant;
pub mod user;

pub use tenant::TenantRepository;
pub use user::UserRepository;