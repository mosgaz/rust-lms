# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- **api**: реализован `PasswordHasher` на базе Argon2id (RFC 9106 compliant, PHC-формат).
- **api**: реализован `JwtManager`, `JwtClaims`, `JwtConfig` для генерации и валидации access/refresh токенов с обязательным claim `tenant_id`.
- **api**: добавлен `AuthService` для координации операций `login` и `refresh` между `UserRepository`, `PasswordHasher` и `JwtManager`.
- **api**: добавлен JWT middleware для извлечения Bearer-токена, валидации и инъекции `TenantId`/`UserId` в `Request::extensions`.
- **api**: реализованы REST-эндпоинты аутентификации: `POST /api/v1/auth/login` и `POST /api/v1/auth/refresh`.
- **api**: добавлены методы `create_with_password` и `find_credentials_by_email` в `UserRepository`.
- **api**: добавлено 28 unit/integration тестов, покрывающих хеширование, JWT, `AuthService`, middleware и маппинг ошибок handlers (покрытие ≥80%).
- **api**: миграция БД `20261005000001_add_password_hash_to_users.sql` (добавление колонки `password_hash`, композитный индекс `(tenant_id, email)`, ограничения длины).
- **shared**: добавлена модель `Credentials` (email + password_hash) для аутентификации.
- **server**: добавлена инициализация `JwtConfig` через переменные окружения (`RUST_LMS_JWT_SECRET`, `RUST_LMS_JWT_ACCESS_TTL`, `RUST_LMS_JWT_REFRESH_TTL`).
- **server**: точка входа Axum-хоста (`main.rs`) с инициализацией `tracing-subscriber`, загрузкой конфигурации, созданием `DatabasePool`, монтированием роутера из `api` и graceful shutdown (SIGINT/SIGTERM).
- **server**: модуль конфигурации (`config.rs`) с загрузкой из `config.toml` и переопределением через переменные окружения `RUST_LMS_*`. Порт по умолчанию: 3720.
- **server**: файл `config.toml` в корне репозитория с настройками сервера и БД.
- **api**: реализован HTTP-слой на базе Axum с разделением на публичные и tenant-scoped маршруты.
- **api**: реализованы REST-обработчики для тенантов (`POST/GET /api/v1/tenants`) и tenant-scoped пользователей (`POST/GET /api/v1/users`).
- **api**: добавлен унифицированный `ApiResponse<T>` для консистентного формата ответов API.
- **api**: реализован `DatabasePool` с конфигурируемыми лимитами соединений.
- **api**: реализован `RlsContext` для установки сессионной переменной `app.current_tenant_id` (ADR 2026.09.28-0001).
- **api**: добавлены базовые репозитории `TenantRepository` и `UserRepository` с принудительным RLS-контекстом.
- **api**: миграция БД `20261003000001_init_rls_and_tenants.sql` с таблицами `tenants`, `users`, политиками RLS и индексами.

### Changed
- **api**: удалён устаревший middleware `extract_tenant_context` (`X-Tenant-ID`), теперь аутентификация и извлечение контекста тенанта происходят исключительно через JWT Bearer token.
- **api**: обновлён handler `create_user` для приёма и хеширования пароля перед сохранением в БД.

### Refactored
- **shared**: реорганизованы модели `tenant` и `user` в директорию `models/`.
- **shared**: добавлены реализации `Display` для `TenantId` и `UserId`.
- **shared**: зависимость `sqlx` сделана опциональной и активируется через фичу `server`, что позволяет компилировать крейт как для сервера, так и для клиента (`wasm32`).

### Docs
- **specs**: добавлен раздел §2.4 в `CODING_STANDARDS.md` (offline-режим `sqlx`).
- **specs**: актуализированы `STRUCTURE.md` и `STATUS.md` с детальным описанием модуля `auth/`, JWT middleware, новых эндпоинтов и обновлённых правил зависимостей.