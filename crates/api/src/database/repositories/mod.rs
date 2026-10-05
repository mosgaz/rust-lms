// crates/api/src/database/repositories/mod.rs
//! Репозитории для работы с сущностями базы данных.
//!
//! Все операции записи выполняются через `sqlx::query_as` с ручным маппингом
//! через derive-макрос `FromRow` (см. CODING_STANDARDS.md §2.4).

pub mod course;
pub mod identity;
pub mod node;
pub mod tenant;
pub mod user;

pub use course::{CourseRepository, CourseRepositoryError};
pub use identity::{IdentityRepository, IdentityRepositoryError};
pub use node::{NodeRepository, NodeRepositoryError};
pub use tenant::{TenantRepository, TenantRepositoryError};
pub use user::{UserRepository, UserRepositoryError};