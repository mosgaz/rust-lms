// crates/server/src/main.rs
//! Точка входа серверного приложения rust-lms.
//!
//! Инициализирует инфраструктуру (tracing, БД), монтирует Axum-роутер
//! и запускает HTTP-сервер с поддержкой graceful shutdown.

use std::net::SocketAddr;

use axum::Router;
use rust_lms_api::{create_router, DatabasePool, JwtConfig};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod config;

use config::AppConfig;

#[tokio::main]
async fn main() {
    // Инициализация структурированного логирования (fail-fast, допустим unwrap).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    tracing::info!("Starting rust-lms server");

    // Загрузка конфигурации (fail-fast).
    let config = AppConfig::load().expect("failed to load application configuration");

    // Инициализация пула соединений с БД (fail-fast).
    let pool_config = rust_lms_api::database::pool::PoolConfig {
        database_url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        min_connections: config.database.min_connections,
    };

    let pool = DatabasePool::new(pool_config)
        .await
        .expect("failed to initialize database connection pool");

    tracing::info!("Database connection pool initialized");

    // Конфигурация JWT (с fallback на значения по умолчанию для разработки).
    let jwt_config = JwtConfig {
        secret: std::env::var("RUST_LMS_JWT_SECRET")
            .unwrap_or_else(|_| "change-me-in-production-min-32-bytes-long-secret!".to_string()),
        access_ttl_secs: std::env::var("RUST_LMS_JWT_ACCESS_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(900), // 15 минут
        refresh_ttl_secs: std::env::var("RUST_LMS_JWT_REFRESH_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(604_800), // 7 дней
    };

    tracing::info!("JWT configuration loaded");

    // Создание и настройка Axum-роутера.
    let app: Router = create_router(pool, jwt_config)
        .layer(TraceLayer::new_for_http())
        .into();

    // Привязка к адресу.
    let addr = SocketAddr::new(
        config.server.host.parse().expect("invalid server host"),
        config.server.port,
    );

    tracing::info!(%addr, "Binding TCP listener");

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    tracing::info!(%addr, "Server started, waiting for connections");

    // Запуск сервера с graceful shutdown.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    tracing::info!("Server shutdown complete");
}

/// Ожидает сигнала завершения (SIGINT/SIGTERM) для graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}