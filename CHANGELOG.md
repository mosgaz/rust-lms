# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- **shared**: добавлены модели `Identity`, `IdentityId` и `IdentityCredentials` для глобального представления личности (ADR 2026.10.05-0011).
- **api**: добавлен `IdentityRepository` для операций с глобальными личностями (`find_credentials_by_email`, `create_with_password`, `update_preferred_tenant`).
- **api**: добавлен `TokenType::Session` в JWT-инфраструктуру для короткоживущих токенов выбора тенанта (TTL 5 мин, без claim `tenant_id`).
- **api**: добавлен REST-эндпоинт `POST /api/v1/auth/select-tenant` для двухшагового потока аутентификации.
- **api**: добавлена поддержка `preferred_tenant_id` для бесшовного автоматического выбора тенанта при последующих входах в систему.
- **api**: добавлен ADR `2026.10.05-0011.md`, документирующий переход на Identity-First архитектуру.
- **api**: реализован `PasswordHasher` на базе Argon2id (RFC 9106 compliant, PHC-формат).
- **api**: реализован `JwtManager`, `JwtClaims`, `JwtConfig` для генерации и валидации токенов.
- **api**: расширен `AuthService` для координации двухшаговой аутентификации (`authenticate`, `select_tenant`, `refresh`, `create_user_in_tenant`).
- **api**: добавлен JWT middleware для извлечения Bearer-токена, валидации и инъекции `IdentityId` и `TenantId` в `Request::extensions`.
- **api**: добавлено 28+ unit/integration тестов, покрывающих хеширование, JWT, `AuthService`, middleware и маппинг ошибок handlers (покрытие ≥80%).
- **server**: добавлена инициализация `JwtConfig` через переменные окружения (`RUST_LMS_JWT_SECRET`, `RUST_LMS_JWT_ACCESS_TTL`, `RUST_LMS_JWT_REFRESH_TTL`).
- **server**: точка входа Axum-хоста (`main.rs`) с инициализацией `tracing-subscriber`, загрузкой конфигурации, созданием `DatabasePool`, монтированием роутера из `api` и graceful shutdown (SIGINT/SIGTERM).
- **server**: модуль конфигурации (`config.rs`) с загрузкой из `config.toml` и переопределением через переменные окружения `RUST_LMS_*`. Порт по умолчанию: 3720.
- **server**: файл `config.toml` в корне репозитория с настройками сервера и БД.
- **api**: реализован HTTP-слой на базе Axum с разделением на публичные и tenant-scoped маршруты.
- **api**: реализованы REST-обработчики для тенантов (`POST/GET /api/v1/tenants`) и tenant-scoped пользователей (`POST/GET /api/v1/users`).
- **api**: добавлен унифицированный `ApiResponse<T>` для консистентного формата ответов API.
- **api**: реализован `DatabasePool` с конфигурируемыми лимитами соединений.
- **api**: реализован `RlsContext` для установки сессионной переменной `app.current_tenant_id` (ADR 2026.09.28-0001).

### Changed
- **[BREAKING CHANGE] api/db**: Переход на Identity-First архитектуру. Миграция `20261003000001_init_rls_and_tenants.sql` полностью переписана: добавлена глобальная таблица `identities` (без RLS), таблица `users` теперь хранит только связь `identity_id` + `tenant_id` (с RLS). Удалена миграция `20261005000001_add_password_hash_to_users.sql`.
- **api**: Поток аутентификации изменён на двухшаговый (`login` → `select_tenant`) с поддержкой автоматического выбора при наличии валидного `preferred_tenant_id`.
- **api**: `UserRepository` рефакторен: методы работы с паролями перенесены в `IdentityRepository`. Добавлены методы `find_active_tenants_for_identity` и `is_user_active_in_tenant`.
- **api**: Handler `create_user` теперь делегирует создание записей в `AuthService::create_user_in_tenant` для атомарного создания и `identity`, и связи `user`.
- **api**: JWT middleware теперь извлекает и проверяет `IdentityId` наряду с `TenantId`.

### Removed
- **api**: удалены методы `UserRepository::create_with_password` и `find_credentials_by_email`.
- **api**: полностью удалён устаревший middleware `extract_tenant_context` (работавший с заголовком `X-Tenant-ID`).
- **shared**: удалена устаревшая модель `Credentials` (заменена на `IdentityCredentials`).

### Refactored
- **shared**: реорганизованы модели в директорию `models/`.
- **shared**: добавлены реализации `Display` для `TenantId`, `UserId` и `IdentityId`.
- **shared**: зависимость `sqlx` сделана опциональной и активируется через фичу `server`, что позволяет компилировать крейт как для сервера, так и для клиента (`wasm32`).

### Docs
- **specs**: добавлен раздел §2.4 в `CODING_STANDARDS.md` (offline-режим `sqlx`).
- **specs**: актуализированы `STRUCTURE.md`, `STATUS.md`, `DB_SCHEMA.md` и `OPEN_API.md` с детальным описанием Identity-First моделей, двухшагового потока аутентификации, новых эндпоинтов и обновлённой схемы БД.