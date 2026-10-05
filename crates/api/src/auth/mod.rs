// crates/api/src/auth/mod.rs
//! Модуль аутентификации: хеширование паролей, JWT-токены, claims.
//!
//! Реализует безопасное хранение учётных данных и выдачу JWT с claim `tenant_id`
//! для последующей интеграции с RLS-интерцептором (ADR 2026.09.28-0001).

pub mod jwt;
pub mod password;
pub mod service;

pub use jwt::{JwtClaims, JwtConfig, JwtManager, TokenType};
pub use password::{PasswordError, PasswordHasher};
pub use service::{AuthService, AuthServiceError, TokenPair};