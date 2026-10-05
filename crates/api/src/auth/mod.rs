// crates/api/src/auth/mod.rs
//! Модуль аутентификации: хеширование паролей, JWT-токены, claims.
//!
//! Реализует Identity-First архитектуру: один email = одна Identity,
//! но несколько User (по одному на тенант).

pub mod jwt;
pub mod password;
pub mod service;

pub use jwt::{JwtClaims, JwtConfig, JwtManager, TokenType};
pub use password::{PasswordError, PasswordHasher};
pub use service::{AuthResult, AuthService, AuthServiceError, TenantInfo, TokenPair};