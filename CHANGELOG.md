# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- **server**: точка входа Axum-хоста (`main.rs`) с инициализацией `tracing-subscriber`, загрузкой конфигурации, созданием `DatabasePool`, монтированием роутера из `api` и graceful shutdown (SIGINT/SIGTERM).
- **server**: модуль конфигурации (`config.rs`) с загрузкой из `config.toml` и переопределением через переменные окружения `RUST_LMS_*`. Порт по умолчанию: 3720.
- **server**: файл `config.toml` в корне репозитория с настройками сервера и БД.
- **api**: реализован HTTP-слой на базе Axum с разделением на публичные и tenant-scoped маршруты.
- **api**: добавлен middleware `extract_tenant_context` для извлечения `TenantId` из заголовка `X-Tenant-ID`.
- **api**: реализованы REST-обработчики для тенантов (`POST/GET /api/v1/tenants`) и tenant-scoped пользователей (`POST/GET /api/v1/users`).
- **api**: добавлен унифицированный `ApiResponse<T>` для консистентного формата ответов API.
- **api**: реализован `DatabasePool` с конфигурируемыми лимитами соединений.
- **api**: реализован `RlsContext` для установки сессионной переменной `app.current_tenant_id` (ADR 2026.09.28-0001).
- **api**: добавлены базовые репозитории `TenantRepository` и `UserRepository` с принудительным RLS-контекстом.
- **api**: миграция БД `20261003000001_init_rls_and_tenants.sql` с таблицами `tenants`, `users`, политиками RLS и индексами.

### Refactored
- **shared**: реорганизованы модели `tenant` и `user` в директорию `models/`.
- **shared**: добавлены реализации `Display` для `TenantId` и `UserId`.

### Docs
- **specs**: добавлен раздел §2.4 в `CODING_STANDARDS.md` (offline-режим `sqlx`).
- **specs**: актуализированы `STRUCTURE.md`, `STATUS.md` и `CHANGELOG.md` после реализации серверного ядра.