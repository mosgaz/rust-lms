// crates/api/src/auth/password.rs
//! Хеширование и верификация паролей через Argon2id.
//!
//! Argon2id выбран как устойчивый к GPU-атакам алгоритм (рекомендация OWASP).
//! Параметры соответствуют минимальным требованиям RFC 9106 для интерактивных сценариев.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString},
    Argon2,
};
use thiserror::Error;

/// Ошибки хеширования/верификации паролей.
#[derive(Debug, Error)]
pub enum PasswordError {
    /// Пароль не прошёл верификацию (неверный пароль или повреждённый хэш).
    #[error("password verification failed")]
    VerificationFailed,
    /// Внутренняя ошибка библиотеки argon2.
    #[error("argon2 error: {0}")]
    Argon2(String),
}

/// Хешировщик паролей на базе Argon2id.
#[derive(Debug, Clone, Copy, Default)]
pub struct PasswordHasher;

impl PasswordHasher {
    /// Создаёт новый экземпляр хешировщика.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Хеширует пароль в формат PHC string (включает соль и параметры).
    ///
    /// # Errors
    ///
    /// Возвращает `PasswordError`, если библиотека argon2 не смогла выполнить хеширование.
    pub fn hash(&self, password: &str) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PasswordError::Argon2(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// Верифицирует пароль против сохранённого PHC-хэша.
    ///
    /// # Errors
    ///
    /// Возвращает `PasswordError::VerificationFailed`, если пароль не совпадает или хэш повреждён.
    pub fn verify(&self, password: &str, hash: &str) -> Result<(), PasswordError> {
        let parsed = PasswordHash::new(hash).map_err(|_| PasswordError::VerificationFailed)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| PasswordError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_success() {
        let hasher = PasswordHasher::new();
        let password = "correct-horse-battery-staple";
        let hash = hasher.hash(password).expect("hashing must succeed");

        assert!(hash.starts_with("$argon2id$"));
        hasher.verify(password, &hash).expect("verification must succeed");
    }

    #[test]
    fn test_verify_wrong_password() {
        let hasher = PasswordHasher::new();
        let hash = hasher.hash("correct-password").expect("hashing must succeed");

        let result = hasher.verify("wrong-password", &hash);
        assert!(matches!(result, Err(PasswordError::VerificationFailed)));
    }

    #[test]
    fn test_verify_corrupted_hash() {
        let hasher = PasswordHasher::new();
        let result = hasher.verify("any-password", "not-a-valid-hash");
        assert!(matches!(result, Err(PasswordError::VerificationFailed)));
    }
}