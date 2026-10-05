// crates/api/src/auth/jwt.rs
//! Управление JWT-токенами: генерация, валидация, claims.
//!
//! Токены содержат обязательный claim `tenant_id` для интеграции с RLS-интерцептором
//! (ADR 2026.09.28-0001). Используются два типа токенов:
//! - **Access token** — короткоживущий (15 мин), для авторизованных запросов.
//! - **Refresh token** — долгоживущий (7 дней), для обновления access token.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rust_lms_shared::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Ошибки работы с JWT.
#[derive(Debug, Error)]
pub enum JwtError {
    /// Не удалось сгенерировать токен.
    #[error("failed to encode token: {0}")]
    EncodingFailed(#[from] jsonwebtoken::errors::Error),
    /// Токен невалиден (истёк, повреждён, неверная подпись).
    #[error("invalid token: {0}")]
    InvalidToken(String),
    /// В токене отсутствует обязательный claim `tenant_id`.
    #[error("missing tenant_id claim in token")]
    MissingTenantId,
    /// В токене отсутствует обязательный claim `sub` (user_id).
    #[error("missing sub claim in token")]
    MissingSubject,
}

/// Тип JWT-токена.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// Короткоживущий access token для авторизованных запросов.
    Access,
    /// Долгоживущий refresh token для обновления access token.
    Refresh,
}

/// Claims (полезная нагрузка) JWT-токена.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject — идентификатор пользователя (UUID as string).
    pub sub: String,
    /// Tenant ID — идентификатор арендатора (UUID as string) для RLS.
    pub tenant_id: String,
    /// Issued at — время выдачи токена (Unix timestamp).
    pub iat: i64,
    /// Expiration — время истечения токена (Unix timestamp).
    pub exp: i64,
    /// JWT ID — уникальный идентификатор токена (для revocation).
    pub jti: String,
    /// Тип токена (access/refresh).
    pub token_type: TokenType,
}

impl JwtClaims {
    /// Извлекает `UserId` из claim `sub`.
    ///
    /// # Errors
    ///
    /// Возвращает `JwtError::MissingSubject`, если claim отсутствует или невалиден.
    pub fn user_id(&self) -> Result<UserId, JwtError> {
        let uuid = Uuid::parse_str(&self.sub).map_err(|e| JwtError::InvalidToken(e.to_string()))?;
        Ok(UserId(uuid))
    }

    /// Извлекает `TenantId` из claim `tenant_id`.
    ///
    /// # Errors
    ///
    /// Возвращает `JwtError::MissingTenantId`, если claim отсутствует или невалиден.
    pub fn tenant_id(&self) -> Result<TenantId, JwtError> {
        let uuid =
            Uuid::parse_str(&self.tenant_id).map_err(|e| JwtError::InvalidToken(e.to_string()))?;
        Ok(TenantId(uuid))
    }
}

/// Конфигурация JWT-менеджера.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Секретный ключ для подписи токенов (HS256). Минимальная длина — 32 байта.
    pub secret: String,
    /// Время жизни access token (в секундах). Рекомендуется 900 (15 мин).
    pub access_ttl_secs: i64,
    /// Время жизни refresh token (в секундах). Рекомендуется 604800 (7 дней).
    pub refresh_ttl_secs: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production-min-32-bytes-long-secret!".to_string(),
            access_ttl_secs: 900,        // 15 минут
            refresh_ttl_secs: 604_800,   // 7 дней
        }
    }
}

/// Менеджер JWT-токенов.
#[derive(Clone)] 
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    config: JwtConfig,
}

impl JwtManager {
    /// Создаёт новый JWT-менеджер с указанной конфигурацией.
    ///
    /// # Panics
    ///
    /// Паникует, если длина секрета менее 32 байт (недостаточно для HS256).
    #[must_use]
    pub fn new(config: JwtConfig) -> Self {
        assert!(
            config.secret.len() >= 32,
            "JWT secret must be at least 32 bytes long for HS256 security"
        );

        let secret_bytes = config.secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(secret_bytes),
            decoding_key: DecodingKey::from_secret(secret_bytes),
            config,
        }
    }

    /// Генерирует access token для указанного пользователя и тенанта.
    ///
    /// # Errors
    ///
    /// Возвращает `JwtError`, если не удалось закодировать токен.
    pub fn generate_access_token(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> Result<String, JwtError> {
        self.generate_token(user_id, tenant_id, TokenType::Access, self.config.access_ttl_secs)
    }

    /// Генерирует refresh token для указанного пользователя и тенанта.
    ///
    /// # Errors
    ///
    /// Возвращает `JwtError`, если не удалось закодировать токен.
    pub fn generate_refresh_token(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> Result<String, JwtError> {
        self.generate_token(user_id, tenant_id, TokenType::Refresh, self.config.refresh_ttl_secs)
    }

    /// Валидирует токен и возвращает claims.
    ///
    /// Проверяет подпись, срок действия и обязательные claims.
    ///
    /// # Errors
    ///
    /// Возвращает `JwtError`, если токен невалиден.
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.required_spec_claims.insert("exp".to_string());
        validation.required_spec_claims.insert("sub".to_string());

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| JwtError::InvalidToken(e.to_string()))?;

        // Проверяем наличие tenant_id
        if token_data.claims.tenant_id.is_empty() {
            return Err(JwtError::MissingTenantId);
        }

        Ok(token_data.claims)
    }

    /// Внутренний метод генерации токена.
    fn generate_token(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        token_type: TokenType,
        ttl_secs: i64,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: user_id.0.to_string(),
            tenant_id: tenant_id.0.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(ttl_secs)).timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> JwtManager {
        JwtManager::new(JwtConfig {
            secret: "test-secret-that-is-at-least-32-bytes-long!".to_string(),
            access_ttl_secs: 900,
            refresh_ttl_secs: 604_800,
        })
    }

    #[test]
    fn test_generate_and_validate_access_token() {
        let manager = test_manager();
        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        let token = manager
            .generate_access_token(user_id, tenant_id)
            .expect("token generation must succeed");

        let claims = manager.validate_token(&token).expect("validation must succeed");

        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.tenant_id().unwrap(), tenant_id);
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_generate_and_validate_refresh_token() {
        let manager = test_manager();
        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        let token = manager
            .generate_refresh_token(user_id, tenant_id)
            .expect("token generation must succeed");

        let claims = manager.validate_token(&token).expect("validation must succeed");
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_validate_invalid_token() {
        let manager = test_manager();
        let result = manager.validate_token("invalid.token.here");
        assert!(matches!(result, Err(JwtError::InvalidToken(_))));
    }

    #[test]
    fn test_validate_expired_token() {
        let manager = JwtManager::new(JwtConfig {
            secret: "test-secret-that-is-at-least-32-bytes-long!".to_string(),
            access_ttl_secs: -120, // Токен "истёк" 2 минуты назад (с запасом на 60-секундный leeway)
            refresh_ttl_secs: 604_800,
        });

        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        let token = manager
            .generate_access_token(user_id, tenant_id)
            .expect("token generation must succeed");

        let result = manager.validate_token(&token);
        assert!(
            matches!(result, Err(JwtError::InvalidToken(_))),
            "Expected InvalidToken error for expired token, got: {:?}",
            result
        );
    }

    #[test]
    #[should_panic(expected = "JWT secret must be at least 32 bytes long")]
    fn test_short_secret_panics() {
        let _ = JwtManager::new(JwtConfig {
            secret: "too-short".to_string(),
            access_ttl_secs: 900,
            refresh_ttl_secs: 604_800,
        });
    }
}