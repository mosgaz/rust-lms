// crates/api/src/database/pool.rs
//! Менеджер пула соединений PostgreSQL.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;

/// Ошибки инициализации пула соединений.
#[derive(Debug, Error)]
pub enum PoolError {
    /// Не удалось подключиться к базе данных.
    #[error("failed to connect to database: {0}")]
    ConnectionFailed(#[from] sqlx::Error),
}

/// Конфигурация пула соединений.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Строка подключения к PostgreSQL.
    pub database_url: String,
    /// Максимальное количество соединений в пуле.
    pub max_connections: u32,
    /// Минимальное количество соединений в пуле.
    pub min_connections: u32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/rust_lms".to_string()),
            max_connections: 10,
            min_connections: 2,
        }
    }
}

/// Менеджер пула соединений PostgreSQL.
#[derive(Debug, Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Создает новый пул соединений с указанной конфигурацией.
    ///
    /// # Errors
    ///
    /// Возвращает `PoolError`, если не удалось подключиться к базе данных.
    pub async fn new(config: PoolConfig) -> Result<Self, PoolError> {
        tracing::info!(
            max_connections = config.max_connections,
            "Initializing database connection pool"
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect(&config.database_url)
            .await?;

        tracing::info!("Database connection pool initialized successfully");

        Ok(Self { pool })
    }

    /// Возвращает ссылку на внутренний пул соединений.
    #[must_use]
    pub fn inner(&self) -> &PgPool {
        &self.pool
    }
}