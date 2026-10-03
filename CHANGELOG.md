# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- **api**: реализован `DatabasePool` с конфигурируемыми лимитами соединений (min/max connections).
- **api**: реализован `RlsContext` для установки сессионной переменной `app.current_tenant_id` через `set_config()` (ADR 2026.09.28-0001, CODING_STANDARDS.md §2.1).
- **api**: добавлены базовые репозитории `TenantRepository` и `UserRepository` с принудительным применением RLS-контекста в транзакциях.
- **api**: миграция БД `20261003000001_init_rls_and_tenants.sql` с таблицами `tenants` и `users`, политиками RLS и индексами.

### Refactored
- **shared**: реорганизованы модели `tenant` и `user` в директорию `models/` для улучшения архитектуры домена и соответствия строгим требованиям документирования (`#![deny(missing_docs)]`).
- **shared**: добавлены реализации `Display` для `TenantId` и `UserId` для корректной работы с `thiserror`.

### Docs
- **specs**: добавлен раздел §2.4 в `CODING_STANDARDS.md`, регламентирующий временное использование runtime-запросов `sqlx` и переход на compile-time проверку через `.sqlx/` кэш.
- **specs**: актуализированы `STRUCTURE.md` и `STATUS.md` после реализации RLS-фундамента в крейте `api`.