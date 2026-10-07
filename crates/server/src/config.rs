// crates/server/src/config.rs
//! Конфигурация серверного приложения.
//!
//! Загружается из `config.toml` с возможностью переопределения через переменные окружения.

use serde::Deserialize;
use thiserror::Error;

/// Ошибки загрузки конфигурации.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Ошибка чтения файла конфигурации.
    #[error("failed to load configuration: {0}")]
    LoadError(#[from] config::ConfigError),
    /// Отсутствует обязательное поле конфигурации.
    #[error("missing required configuration field: {0}")]
    #[allow(dead_code)]
    MissingField(String),
}

/// Конфигурация HTTP-сервера.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Хост для привязки (например, "0.0.0.0").
    pub host: String,
    /// Порт для прослушивания.
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3720,
        }
    }
}

/// Конфигурация подключения к базе данных.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Строка подключения к PostgreSQL.
    pub url: String,
    /// Максимальное количество соединений в пуле.
    pub max_connections: u32,
    /// Минимальное количество соединений в пуле.
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/rust_lms".to_string(),
            max_connections: 10,
            min_connections: 2,
        }
    }
}

/// Конфигурация SMTP-сервера для отправки уведомлений.
#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    /// Адрес SMTP-сервера (например, `smtp.gmail.com`).
    pub host: String,
    /// Порт SMTP-сервера (обычно 587 для STARTTLS или 465 для TLS).
    pub port: u16,
    /// Имя пользователя для аутентификации на SMTP-сервере.
    pub username: String,
    /// Пароль или App Password для аутентификации.
    pub password: String,
    /// Адрес отправителя (From), отображаемый в письмах.
    pub from_address: String,
    /// Имя отправителя, отображаемое в письмах.
    pub from_name: String,
}

/// Корневая конфигурация приложения.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Конфигурация HTTP-сервера.
    pub server: ServerConfig,
    /// Конфигурация базы данных.
    pub database: DatabaseConfig,
	/// Конфигурация SMTP для отправки уведомлений.
    pub smtp: SmtpConfig,
}

impl AppConfig {
    /// Загружает конфигурацию из `config.toml` с переопределением через переменные окружения.
    ///
    /// Приоритет источников (от низшего к высшему):
    /// 1. Значения по умолчанию.
    /// 2. Файл `config.toml` (если существует).
    /// 3. Переменные окружения с префиксом `RUST_LMS_` (например, `RUST_LMS_SERVER__PORT`).
    ///
    /// # Errors
    ///
    /// Возвращает `ConfigError`, если не удалось прочитать или распарсить конфигурацию.
    pub fn load() -> Result<Self, ConfigError> {
        let builder = config::Config::builder()
            // Файл конфигурации (опциональный)
            .add_source(config::File::with_name("config").required(false))
            // Переменные окружения с префиксом RUST_LMS_ и разделителем __
            .add_source(
                config::Environment::with_prefix("RUST_LMS")
                    .separator("__")
                    .try_parsing(true),
            );

        let config = builder.build()?;
        let app_config: Self = config.try_deserialize()?;

        tracing::info!(
            host = %app_config.server.host,
            port = app_config.server.port,
            "Configuration loaded successfully"
        );

        Ok(app_config)
    }
}