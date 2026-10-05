// crates/api/src/auth/jwt.rs
//! Управление JWT-токенами: генерация, валидация, claims.
//!
//! Токены содержат обязательный claim `tenant_id` (кроме Session токена) 
//! для интеграции с RLS-интерцептором (ADR 2026.09.28-0001).

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rust_lms_shared::{IdentityId, TenantId};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Ошибки работы с JWT.
#[derive(Debug, Error)]
pub enum JwtError {
    /// Не удалось закодировать токен.
    #[error("failed to encode token: {0}")]
    EncodingFailed(#[from] jsonwebtoken::errors::Error),
    /// Токен невалиден (истёк, повреждён, неверная подпись).
    #[error("invalid token: {0}")]
    InvalidToken(String),
    /// В токене отсутствует обязательный claim `tenant_id`.
    #[error("missing tenant_id claim in token")]
    MissingTenantId,
    /// В токене отсутствует обязательный claim `sub`.
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
    /// Короткоживущий session token для выбора тенанта (без tenant_id).
    Session,
}

/// Claims (полезная нагрузка) JWT-токена.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject — идентификатор личности (IdentityId as string).
    pub sub: String,
    /// Tenant ID — идентификатор арендатора (UUID as string) для RLS.
    pub tenant_id: String,
    /// Issued at — время выдачи токена (Unix timestamp).
    pub iat: i64,
    /// Expiration — время истечения токена (Unix timestamp).
    pub exp: i64,
    /// JWT ID — уникальный идентификатор токена.
    pub jti: String,
    /// Тип токена (access/refresh/session).
    pub token_type: TokenType,
}

impl JwtClaims {
    /// Извлекает `IdentityId` из claim `sub`.
    pub fn identity_id(&self) -> Result<IdentityId, JwtError> {
        let uuid = Uuid::parse_str(&self.sub).map_err(|e| JwtError::InvalidToken(e.to_string()))?;
        Ok(IdentityId(uuid))
    }

    /// Извлекает `TenantId` из claim `tenant_id`.
    pub fn tenant_id(&self) -> Result<TenantId, JwtError> {
        if self.tenant_id.is_empty() {
            return Err(JwtError::MissingTenantId);
        }
        let uuid = Uuid::parse_str(&self.tenant_id).map_err(|e| JwtError::InvalidToken(e.to_string()))?;
        Ok(TenantId(uuid))
    }
}

/// Конфигурация JWT-менеджера.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Секретный ключ для подписи токенов (HS256). Минимальная длина — 32 байта.
    pub secret: String,
    /// Время жизни access token (в секундах).
    pub access_ttl_secs: i64,
    /// Время жизни refresh token (в секундах).
    pub refresh_ttl_secs: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production-min-32-bytes-long-secret!".to_string(),
            access_ttl_secs: 900,
            refresh_ttl_secs: 604_800,
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
    #[must_use]
    pub fn new(config: JwtConfig) -> Self {
        assert!(config.secret.len() >= 32, "JWT secret must be at least 32 bytes long for HS256");
        let secret_bytes = config.secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(secret_bytes),
            decoding_key: DecodingKey::from_secret(secret_bytes),
            config,
        }
    }

    /// Генерирует access token для указанной личности и тенанта.
    pub fn generate_access_token(&self, identity_id: IdentityId, tenant_id: TenantId) -> Result<String, JwtError> {
        self.generate_token(identity_id, tenant_id, TokenType::Access, self.config.access_ttl_secs)
    }

    /// Генерирует refresh token для указанной личности и тенанта.
    pub fn generate_refresh_token(&self, identity_id: IdentityId, tenant_id: TenantId) -> Result<String, JwtError> {
        self.generate_token(identity_id, tenant_id, TokenType::Refresh, self.config.refresh_ttl_secs)
    }

    /// Генерирует session token для выбора тенанта (без tenant_id, TTL 5 мин).
    pub fn generate_session_token(&self, identity_id: IdentityId) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: identity_id.0.to_string(),
            tenant_id: String::new(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(300)).timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Session,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(Into::into)
    }

    /// Валидирует токен и возвращает claims.
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.required_spec_claims.insert("exp".to_string());
        validation.required_spec_claims.insert("sub".to_string());

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| JwtError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    fn generate_token(&self, identity_id: IdentityId, tenant_id: TenantId, token_type: TokenType, ttl_secs: i64) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = JwtClaims {
            sub: identity_id.0.to_string(),
            tenant_id: tenant_id.0.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(ttl_secs)).timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(Into::into)
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
        let identity_id = IdentityId::new();
        let tenant_id = TenantId::new();

        let token = manager.generate_access_token(identity_id, tenant_id).expect("token generation must succeed");
        let claims = manager.validate_token(&token).expect("validation must succeed");

        assert_eq!(claims.identity_id().unwrap(), identity_id);
        assert_eq!(claims.tenant_id().unwrap(), tenant_id);
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_generate_and_validate_session_token() {
        let manager = test_manager();
        let identity_id = IdentityId::new();

        let token = manager.generate_session_token(identity_id).expect("token generation must succeed");
        let claims = manager.validate_token(&token).expect("validation must succeed");

        assert_eq!(claims.identity_id().unwrap(), identity_id);
        assert_eq!(claims.token_type, TokenType::Session);
        assert!(claims.tenant_id().is_err());
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
            access_ttl_secs: -120,
            refresh_ttl_secs: 604_800,
        });

        let identity_id = IdentityId::new();
        let tenant_id = TenantId::new();
        let token = manager.generate_access_token(identity_id, tenant_id).expect("token generation must succeed");

        let result = manager.validate_token(&token);
        assert!(matches!(result, Err(JwtError::InvalidToken(_))));
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