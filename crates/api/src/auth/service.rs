// crates/api/src/auth/service.rs
//! Сервис аутентификации для Identity-First архитектуры.

use rust_lms_shared::{IdentityId, TenantId};
use serde::Serialize;
use thiserror::Error;

use crate::database::{
    IdentityRepository, IdentityRepositoryError, TenantRepository, TenantRepositoryError,
    UserRepository, UserRepositoryError,
};

use super::jwt::{JwtManager, TokenType};
use super::password::{PasswordError, PasswordHasher};

/// Ошибки сервиса аутентификации.
#[derive(Debug, Error)]
pub enum AuthServiceError {
    /// Неверный email или пароль.
    #[error("invalid credentials")]
    InvalidCredentials,
    /// Пользователь деактивирован в выбранном тенанте.
    #[error("user account is disabled in this tenant")]
    AccountDisabled,
    /// Тенант не найден или деактивирован.
    #[error("tenant not found or inactive")]
    TenantNotFound,
    /// У пользователя нет доступа к выбранному тенанту.
    #[error("access denied to this tenant")]
    AccessDenied,
    /// Email уже зарегистрирован в системе.
    #[error("email already registered in the system")]
    EmailAlreadyExists,
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(String),
    /// Ошибка генерации JWT.
    #[error("token generation failed: {0}")]
    TokenGenerationFailed(String),
    /// Невалидный session token.
    #[error("invalid session token")]
    InvalidSessionToken,
}

impl From<IdentityRepositoryError> for AuthServiceError {
    fn from(e: IdentityRepositoryError) -> Self {
        match e {
            IdentityRepositoryError::NotFound(_) => AuthServiceError::InvalidCredentials,
            IdentityRepositoryError::EmailAlreadyExists(_) => AuthServiceError::EmailAlreadyExists,
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

impl From<UserRepositoryError> for AuthServiceError {
    fn from(e: UserRepositoryError) -> Self {
        match e {
            UserRepositoryError::NotFound(_) => AuthServiceError::AccessDenied,
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
    /// Короткоживущий access token.
    pub access_token: String,
    /// Долгоживущий refresh token.
    pub refresh_token: String,
}

/// Информация о доступном тенанте для выбора.
#[derive(Debug, Clone, Serialize)]
pub struct TenantInfo {
    /// Идентификатор тенанта.
    pub id: TenantId,
    /// Название тенанта.
    pub name: String,
    /// Slug тенанта.
    pub slug: String,
}

/// Результат аутентификации.
#[derive(Debug)]
pub enum AuthResult {
    /// Автоматический вход в единственный/предпочитаемый тенант.
    SingleTenant(TokenPair),
    /// Необходим выбор тенанта из списка.
    MultiTenant {
        /// Короткоживущий session token.
        session_token: String,
        /// Список доступных тенантов.
        available_tenants: Vec<TenantInfo>,
    },
}

/// Сервис аутентификации.
#[derive(Clone)]
pub struct AuthService {
    identity_repo: IdentityRepository,
    user_repo: UserRepository,
    tenant_repo: TenantRepository,
    hasher: PasswordHasher,
    jwt: JwtManager,
}

impl AuthService {
    /// Создаёт новый сервис аутентификации.
    #[must_use]
    pub fn new(
        identity_repo: IdentityRepository,
        user_repo: UserRepository,
        tenant_repo: TenantRepository,
        jwt: JwtManager,
    ) -> Self {
        Self {
            identity_repo,
            user_repo,
            tenant_repo,
            hasher: PasswordHasher::new(),
            jwt,
        }
    }

    /// Аутентифицирует пользователя по email и паролю.
    pub async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthServiceError> {
        let credentials = self.identity_repo.find_credentials_by_email(email).await?;
        self.hasher.verify(password, &credentials.password_hash)?;

        let tenant_ids = self.user_repo.find_active_tenants_for_identity(credentials.identity_id).await?;
        if tenant_ids.is_empty() {
            return Err(AuthServiceError::AccessDenied);
        }

        if let Some(preferred_tenant_id) = credentials.preferred_tenant_id {
            if tenant_ids.contains(&preferred_tenant_id) {
                let tenant = self.tenant_repo.find_by_id(preferred_tenant_id).await?;
                if tenant.is_active {
                    let token_pair = self.generate_token_pair(credentials.identity_id, preferred_tenant_id).await?;
                    return Ok(AuthResult::SingleTenant(token_pair));
                }
            }
        }

        let mut available_tenants = Vec::new();
        for tenant_id in tenant_ids {
            let tenant = self.tenant_repo.find_by_id(tenant_id).await?;
            if tenant.is_active {
                available_tenants.push(TenantInfo { id: tenant.id, name: tenant.name, slug: tenant.slug });
            }
        }

        if available_tenants.is_empty() {
            return Err(AuthServiceError::AccessDenied);
        }

        let session_token = self.generate_session_token(credentials.identity_id)?;
        Ok(AuthResult::MultiTenant { session_token, available_tenants })
    }

    /// Выбирает конкретный тенант и выдаёт финальные токены.
    pub async fn select_tenant(&self, session_token: &str, tenant_id: TenantId) -> Result<TokenPair, AuthServiceError> {
        let claims = self.jwt.validate_token(session_token).map_err(|_| AuthServiceError::InvalidSessionToken)?;
        if claims.token_type != TokenType::Session {
            return Err(AuthServiceError::InvalidSessionToken);
        }

        let identity_id = claims.identity_id().map_err(|_| AuthServiceError::InvalidSessionToken)?;
        let is_active = self.user_repo.is_user_active_in_tenant(identity_id, tenant_id).await?;
        if !is_active {
            return Err(AuthServiceError::AccessDenied);
        }

        let tenant = self.tenant_repo.find_by_id(tenant_id).await?;
        if !tenant.is_active {
            return Err(AuthServiceError::TenantNotFound);
        }

        let token_pair = self.generate_token_pair(identity_id, tenant_id).await?;
        self.identity_repo.update_preferred_tenant(identity_id, Some(tenant_id)).await
            .map_err(|e| AuthServiceError::DatabaseError(e.to_string()))?;

        Ok(token_pair)
    }

    /// Обновляет access token по валидному refresh token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthServiceError> {
        let claims = self.jwt.validate_token(refresh_token).map_err(|_| AuthServiceError::InvalidSessionToken)?;
        if claims.token_type != TokenType::Refresh {
            return Err(AuthServiceError::InvalidSessionToken);
        }

        let identity_id = claims.identity_id().map_err(|_| AuthServiceError::InvalidSessionToken)?;
        let tenant_id = claims.tenant_id().map_err(|_| AuthServiceError::InvalidSessionToken)?;

        let tenant = self.tenant_repo.find_by_id(tenant_id).await?;
        if !tenant.is_active {
            return Err(AuthServiceError::TenantNotFound);
        }

        let is_active = self.user_repo.is_user_active_in_tenant(identity_id, tenant_id).await?;
        if !is_active {
            return Err(AuthServiceError::AccountDisabled);
        }

        self.generate_token_pair(identity_id, tenant_id).await
    }

    /// Создаёт новую личность и связывает её с тенантом.
    pub async fn create_user_in_tenant(&self, tenant_id: TenantId, email: &str, password: &str) -> Result<rust_lms_shared::User, AuthServiceError> {
        let password_hash = self.hasher.hash(password).map_err(|e| AuthServiceError::DatabaseError(e.to_string()))?;
        let identity_id = self.identity_repo.create_with_password(email, &password_hash).await?;
        let user = self.user_repo.create(tenant_id, identity_id).await?;
        Ok(user)
    }

    async fn generate_token_pair(&self, identity_id: IdentityId, tenant_id: TenantId) -> Result<TokenPair, AuthServiceError> {
        let access_token = self.jwt.generate_access_token(identity_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;
        let refresh_token = self.jwt.generate_refresh_token(identity_id, tenant_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))?;
        Ok(TokenPair { access_token, refresh_token })
    }

    fn generate_session_token(&self, identity_id: IdentityId) -> Result<String, AuthServiceError> {
        self.jwt.generate_session_token(identity_id)
            .map_err(|e| AuthServiceError::TokenGenerationFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_credentials() {
        assert_eq!(AuthServiceError::InvalidCredentials.to_string(), "invalid credentials");
    }

    #[test]
    fn test_from_identity_repo_not_found() {
        let err: AuthServiceError = IdentityRepositoryError::NotFound(IdentityId::new()).into();
        assert!(matches!(err, AuthServiceError::InvalidCredentials));
    }

    #[test]
    fn test_from_password_error() {
        let err: AuthServiceError = PasswordError::VerificationFailed.into();
        assert!(matches!(err, AuthServiceError::InvalidCredentials));
    }
}