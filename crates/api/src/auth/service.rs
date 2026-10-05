// crates/api/src/auth/service.rs
//! Сервис аутентификации: координирует UserRepository, PasswordHasher и JwtManager.
//!
//! Выступает единой точкой входа для операций login/refresh, инкапсулируя
//! бизнес-логику проверки паролей и генерации токенов.

use rust_lms_shared::{Tenant, TenantId};
use thiserror::Error;

use crate::database::{TenantRepository, TenantRepositoryError, UserRepository, UserRepositoryError};

use super::jwt::JwtManager;
use super::password::{PasswordError, PasswordHasher};

/// Ошибки сервиса аутентификации.
#[derive(Debug, Error)]
pub enum AuthServiceError {
    /// Неверный email или пароль (намеренно не детализируем для безопасности).
    #[error("invalid credentials")]
    InvalidCredentials,
    /// Пользователь деактивирован.
    #[error("user account is disabled")]
    AccountDisabled,
    /// Тенант не найден или деактивирован.
    #[error("tenant not found or inactive")]
    TenantNotFound,
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(String),
    /// Ошибка генерации JWT.
    #[error("token generation failed: {0}")]
    TokenGenerationFailed(String),
    /// Невалидный refresh token.
    #[error("invalid refresh token")]
    InvalidRefreshToken,
}

impl From<UserRepositoryError> for AuthServiceError {
    fn from(e: UserRepositoryError) -> Self {
        match e {
            UserRepositoryError::NotFound(_) => AuthServiceError::InvalidCredentials,
            other => AuthServiceError::DatabaseError(other.to_string()),
        }
    }
}

impl From<TenantRepositoryError> for AuthServiceError {
    fn from(e: TenantRepositoryError) -> Self {
        match e {
            TenantRepositoryError::NotFound(_) => AuthServiceError::TenantNotFound,
            other => AuthServiceError::DatabaseError(other.to_string()),
        }
    }
}

impl From<PasswordError> for AuthServiceError {
    fn from(_: PasswordError) -> Self {
        AuthServiceError::InvalidCredentials
    }
}

/// Пара токенов: access + refresh.
#[derive(Debug, Clone)]
pub struct TokenPair {
    /// Короткоживущий access token (для авторизованных запросов).
    pub access_token: String,
    /// Долгоживущий refresh token (для обновления access token).
    pub refresh_token: String,
}

/// Сервис аутентификации.
#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    tenant_repo: TenantRepository,
    hasher: PasswordHasher,
    jwt: JwtManager,
}

impl AuthService {
    /// Создаёт новый сервис аутентификации.
    #[must_use]
    pub fn new(
        user_repo: UserRepository,
        tenant_repo: TenantRepository,
        jwt: JwtManager,
    ) -> Self {
        Self {
            user_repo,
            tenant_repo,
            hasher: PasswordHasher::new(),
            jwt,
        }
    }

    /// Аутентифицирует пользователя по email и паролю, возвращает пару токенов.
    ///
    /// # Errors
    ///
    /// Возвращает `AuthServiceError::InvalidCredentials`, если email/пароль неверны
    /// или пользователь деактивирован. Возвращает `AuthServiceError::TenantNotFound`,
    /// если тенант не найден или деактивирован.
    pub async fn login(
        &self,
        tenant_id: TenantId,
        email: &str,
        password: &str,
    ) -> Result<TokenPair, AuthServiceError> {
        // 1. Проверяем, что тенант активен
        let tenant: Tenant = self.tenant_repo.find_by_id(tenant_id).await?;
        if !tenant.is_active {
            return Err(AuthServiceError::TenantNotFound);
        }

        // 2. Получаем учётные данные пользователя
        let credentials = self.user_repo.find_credentials_by_email(tenant_id, email).await?;

        // 3. Проверяем активность пользователя
        if !credentials.is_active {
            return Err(AuthServiceError::AccountDisabled);
        }

        // 4. Верифицируем пароль
        self.hasher.verify(password, &credentials.password_hash)?;

        // 5. Генерируем пару токенов
        let access_token = self
            .jwt
            .generate_access_token(credentials.user_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;

        let refresh_token = self
            .jwt
            .generate_refresh_token(credentials.user_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;

        tracing::info!(
            user_id = %credentials.user_id,
            tenant_id = %tenant_id,
            "User authenticated successfully"
        );

        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    /// Обновляет access token по валидному refresh token.
    ///
    /// # Errors
    ///
    /// Возвращает `AuthServiceError::InvalidRefreshToken`, если refresh token невалиден,
    /// истёк или не является refresh-токеном.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthServiceError> {
        // 1. Валидируем refresh token
        let claims = self
            .jwt
            .validate_token(refresh_token)
            .map_err(|_| AuthServiceError::InvalidRefreshToken)?;

        // 2. Проверяем, что это именно refresh token
        if claims.token_type != super::jwt::TokenType::Refresh {
            return Err(AuthServiceError::InvalidRefreshToken);
        }

        let user_id = claims.user_id().map_err(|_| AuthServiceError::InvalidRefreshToken)?;
        let tenant_id = claims.tenant_id().map_err(|_| AuthServiceError::InvalidRefreshToken)?;

        // 3. Проверяем активность тенанта и пользователя
        let tenant = self.tenant_repo.find_by_id(tenant_id).await?;
        if !tenant.is_active {
            return Err(AuthServiceError::TenantNotFound);
        }

        let user = self.user_repo.find_by_id(tenant_id, user_id).await?;
        if !user.is_active {
            return Err(AuthServiceError::AccountDisabled);
        }

        // 4. Генерируем новую пару токенов
        let access_token = self
            .jwt
            .generate_access_token(user_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;

        let new_refresh_token = self
            .jwt
            .generate_refresh_token(user_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;

        tracing::info!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            "Token pair refreshed"
        );

        Ok(TokenPair {
            access_token,
            refresh_token: new_refresh_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_lms_shared::UserId;

    #[test]
    fn test_error_display_invalid_credentials() {
        let err = AuthServiceError::InvalidCredentials;
        assert_eq!(err.to_string(), "invalid credentials");
    }

    #[test]
    fn test_error_display_account_disabled() {
        let err = AuthServiceError::AccountDisabled;
        assert_eq!(err.to_string(), "user account is disabled");
    }

    #[test]
    fn test_error_display_tenant_not_found() {
        let err = AuthServiceError::TenantNotFound;
        assert_eq!(err.to_string(), "tenant not found or inactive");
    }

    #[test]
    fn test_error_display_invalid_refresh_token() {
        let err = AuthServiceError::InvalidRefreshToken;
        assert_eq!(err.to_string(), "invalid refresh token");
    }

    #[test]
    fn test_from_user_repo_not_found() {
        let user_err = UserRepositoryError::NotFound(UserId::new());
        let auth_err: AuthServiceError = user_err.into();
        assert!(matches!(auth_err, AuthServiceError::InvalidCredentials));
    }

    #[test]
    fn test_from_tenant_repo_not_found() {
        let tenant_err = TenantRepositoryError::NotFound(TenantId::new());
        let auth_err: AuthServiceError = tenant_err.into();
        assert!(matches!(auth_err, AuthServiceError::TenantNotFound));
    }

    #[test]
    fn test_from_password_error() {
        let pwd_err = PasswordError::VerificationFailed;
        let auth_err: AuthServiceError = pwd_err.into();
        assert!(matches!(auth_err, AuthServiceError::InvalidCredentials));
    }
}