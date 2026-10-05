# Текущий статус реализации системы (Feature Status Matrix)

**Файл спецификации:** `specs/STATUS.md`

> Этот файл динамически обновляется AI-агентами после завершения каждого таска. Изменение статусов дублируется записью в `CHANGELOG.md`.

**Текущий этап проекта:** активная реализация серверного ядра (`api` + `server`). Базовый HTTP-цикл (DTO → БД → REST → Axum-хост) замкнут. **Внедрена Identity-First архитектура** (ADR 2026.10.05-0011): глобальная личность (`identities`) + роли в тенантах (`users`), двухшаговая аутентификация с авто-выбором `preferred_tenant_id`.
**Кодовая база:** активная разработка.

---

## Легенда статусов

### Легенда A: Проектирование и документация
- 🟢 **УТВЕРЖДЕНО** — документ согласован, является источником истины.
- 🟡 **ЧЕРНОВИК** — документ в процессе написания или на ревью.
- 🔴 **НЕ НАЧАТО** — документация отсутствует.

### Легенда B: Реализация (Код)
- 🟢 **ГОТОВО** — бизнес-логика написана, покрыта тестами ≥80%, RLS-проверки активны (или эквивалент для клиента).
- 🟡 **ЗАГЛУШКА / ЧАСТИЧНО** — интерфейсы объявлены, базовая логика работает (например, через mock), но полная интеграция с бэкендом или смежными системами ожидается.
- 🔴 **НЕ СДЕЛАНО** — функционал отсутствует, контракты не объявлены.

### Легенда C: Планы
- ⚪ **ПЛАН** — зафиксировано в спецификации (`ROADMAP.md`), реализация не начата и не декомпозирована на таски.

---

## 📐 Проектирование и документация (Легенда A)

| Артефакт | Статус | Ответственный | Примечания |
| :-- | :-: | :-- | :-- |
| `specs/README.md` | 🟢 | — | Разводящая страница документации. |
| `specs/SPECIFICATION.md` | 🟢 | — | Бизнес-концепция, иерархия, версионирование, retention, роли. |
| `specs/ARCHITECTURE.md` | 🟢 | — | Сводный ADD: RLS, Open API, LRS, плагины, ETL. |
| `specs/NFR.md` | 🟢 | — | SLA, RTO/RPO, concurrency, latency budgets, лимиты. |
| `specs/STRUCTURE.md` | 🟢 | — | Карта папок, Dependency Rules, состав крейтов (актуализировано под Identity-First). |
| `specs/DB_SCHEMA.md` | 🟢 | — | Таблицы, RLS, версионирование, i18n, retention, LRS, feature flags, license (актуализировано: `identities` + `users`). |
| `specs/MIGRATIONS.md` | 🟢 | — | Регламент на базе `sqlx` + Runbook для администратора. |
| `specs/OPEN_API.md` | 🟢 | — | REST/GraphQL, Opaque-токены, SCIM 2.0, signed-url, вебхуки, **offline-sync**, двухшаговая аутентификация. |
| `specs/OFFLINE_SYNC.md` | 🟢 | — | IndexedDB, синхронизация, конфликты, iOS-лимиты, DRM, офлайн-шелл. |
| `specs/PLUGIN.md` | 🟢 | — | Двухуровневый рантайм, FSM, подпись и kill switch. |
| `specs/PLUGIN_DEVELOPMENT_TEMPLATE.md` | 🟢 | — | Шаблон ТЗ для внешних команд (включая a11y). |
| `specs/DEPLOY.md` | 🟢 | — | Docker Compose, Ingress, CSP, COOP/COEP, Air-gapped, ключи, DR. |
| `specs/LICENSING.md` | 🟢 | — | Офлайн-лицензирование для коробочных поставок. |
| `specs/FEATURE_FLAGS.md` | 🟢 | — | Управление функциональными флагами (глобальные + tenant overrides). |
| `specs/STANDARDS.md` | 🟢 | — | SCORM, xAPI, LTI, SCIM 2.0, WCAG 2.2 AA, i18n, GDPR, Data Portability. |
| `specs/COMMUNICATIONS.md` | 🟢 | — | Чаты, комментарии, уведомления. |
| `specs/CONFERENCING.md` | 🟢 | — | WebRTC P2P/SFU, fallback, ограничения MVP, локальные TURN/STUN. |
| `specs/ROADMAP.md` | 🟢 | — | Планы по аналитике, сертификации, биллингу, поиску, мобильному, аудиту. |
| `specs/RBAC.md` | 🟢 | — | Матрица ролей и доступов. |
| `specs/DIAGNOSTICS.md` | 🟢 | — | Логирование (`tracing`) и распределённый трейсинг (`OpenTelemetry`). |
| `specs/GOTCHAS.md` | 🟢 | — | Журнал технических ловушек. |
| `specs/CODING_STANDARDS.md` | 🟢 | — | Правила full-stack Rust, RLS, запреты, `sqlx` vs `SeaORM` (добавлен §2.4 про offline-режим). |
| `specs/AGENTS.md` | 🟢 | — | Инструкции для AI-агентов. |
| `specs/decisions/README.md` | 🟢 | — | Реестр ADR, точка входа. |
| `specs/decisions/2026.09.28-0001.md` | 🟢 | — | ADR: RLS вместо схем-per-tenant. |
| `specs/decisions/2026.09.28-0002.md` | 🟢 | — | ADR: иммутабельный xAPI в LRS. |
| `specs/decisions/2026.09.29-0003.md` | 🟢 | — | ADR: подпись и kill switch для WASM-плагинов. |
| `specs/decisions/2026.09.29-0004.md` | 🟢 | — | ADR: Application-Level Encryption. |
| `specs/decisions/2026.09.29-0005.md` | 🟢 | — | ADR: Data Residency. |
| `specs/decisions/2026.09.29-0006.md` | 🟢 | — | ADR: выбор OTel backend для SaaS. |
| `specs/decisions/2026.09.29-0007.md` | 🟢 | — | ADR: операционный регламент deprecation API. |
| `specs/decisions/2026.09.29-0008.md` | 🟢 | — | ADR: формат и enforcement лицензионного ключа. |
| `specs/decisions/2026.09.29-0009.md` | 🟢 | — | ADR: архитектура Feature Flags. |
| `specs/decisions/2026.09.29-0010.md` | 🟢 | — | ADR: Supply Chain Security для WASM-плагинов. |
| `specs/decisions/2026.10.05-0011.md` | 🟢 | — | ADR: Identity-First архитектура (разделение личности и роли в тенанте). |
| `CONTRIBUTING.md` | 🟢 | — | Коммиты, ветвление, Conventional Commits. |
| `CHANGELOG.md` | 🟢 | — | Журнал изменений. |

---

## 📊 Матрица слоёв и фич: Реализация (Легенда B)

| Компонент / Фича | Тип | Статус | Крейт-ответственный | Примечания / Ссылка на ADR |
| :-- | :-: | :-: | :-- | :-- |
| **Базовые DTO сущностей (Identity, User, Tenant)** | Реализация | 🟢 | `shared` | Identity-First модели: `Identity` (глобальная личность), `User` (связь identity-tenant), `Tenant`. Типобезопасные ID (`IdentityId`, `UserId`, `TenantId`). ADR: `2026.10.05-0011.md`. |
| **Identity-First схема БД** | Реализация | 🟢 | `api` | Миграция `20261003000001`: таблицы `tenants`, `identities` (без RLS), `users` (с RLS). CHECK constraints, индексы. ADR: `2026.10.05-0011.md`. |
| **IdentityRepository** | Реализация | 🟢 | `api` | CRUD для глобальных личностей: `find_credentials_by_email`, `find_by_id`, `update_preferred_tenant`, `create_with_password`. Unit-тесты. |
| **UserRepository (Identity-First)** | Реализация | 🟢 | `api` | CRUD для связей identity-tenant: `find_active_tenants_for_identity`, `is_user_active_in_tenant`, `create`, `find_by_id`. Unit-тесты. |
| **Мультиарендность (Strict Multi-tenancy)** | Реализация | 🟢 | `api` | RLS-интерцептор, репозитории и Axum middleware (JWT Bearer → `IdentityId` + `TenantId` из claims). ADR: `2026.09.28-0001.md`. |
| **Хеширование паролей (Argon2id)** | Реализация | 🟢 | `api` | `PasswordHasher` с PHC-форматом, RFC 9106 compliant. Хэш хранится в `identities.password_hash`. 3 unit-теста. |
| **JWT-инфраструктура (Access/Refresh/Session)** | Реализация | 🟢 | `api` | `JwtManager`, `JwtClaims` с claim `tenant_id`, `TokenType` (Access/Refresh/Session). Session token — без `tenant_id`, TTL 5 мин. Конфигурация через env `RUST_LMS_JWT_*`. 5 unit-тестов. |
| **AuthService (двухшаговая аутентификация)** | Реализация | 🟢 | `api` | `authenticate` → `AuthResult::SingleTenant` (авто-выбор по `preferred_tenant_id`) или `AuthResult::MultiTenant` (session_token + список). `select_tenant` — выбор тенанта, обновление `preferred_tenant_id`. `refresh`, `create_user_in_tenant`. 3 unit-теста. |
| **preferred_tenant_id (авто-выбор тенанта)** | Реализация | 🟢 | `api` | Глобальное поле в `identities`, обновляется при `select_tenant`. Используется для бесшовного входа при следующем логине. ADR: `2026.10.05-0011.md`. |
| **JWT middleware (Bearer auth)** | Реализация | 🟢 | `api` | Извлечение Bearer-токена, валидация, инъекция `IdentityId`/`TenantId` в extensions. Отклонение refresh/session токенов для защищённых маршрутов. 2 integration-теста. |
| **REST-эндпоинты аутентификации** | Реализация | 🟢 | `api` | `POST /api/v1/auth/login`, `POST /api/v1/auth/select-tenant`, `POST /api/v1/auth/refresh`. Унифицированный `ApiResponse<T>`. 2 unit-теста. |
| **Динамический Provisioning тенантов** | Реализация | 🟡 | `api` | REST-эндпоинты `POST/GET /api/v1/tenants` реализованы (заглушки). Ожидает расширения (обновление, деактивация, SCIM). |
| **Точка входа сервера (Axum Host)** | Реализация | 🟢 | `server` | `main.rs`: tracing, config, DatabasePool, **JwtConfig**, роутер из `api`, TraceLayer, graceful shutdown. Порт 3720. |
| **Конфигурация приложения** | Реализация | 🟢 | `server` | `config.toml` + переменные окружения `RUST_LMS_*` (включая `RUST_LMS_JWT_SECRET`, `RUST_LMS_JWT_ACCESS_TTL`, `RUST_LMS_JWT_REFRESH_TTL`). |
| **Слой открытых токенов и API Keys** | Реализация | 🔴 | `api` | SHA-256 хэширование Opaque-ключей в БД. |
| **Конвейер кастомного импорта (ETL)** | Реализация | 🔴 | `api` | Потоковый парсинг CSV/XLSX чанками. |
| **SCIM 2.0 (Users/Groups)** | Реализация | 🔴 | `api` | Real-time provisioning из HRIS/HRM. |
| **Иерархия (Programs / Courses / Batches)** | Реализация | 🟡 | `shared` / `api` | Объявление плоских DTO-контрактов (файлы-заглушки созданы), базовые сущности в работе. |
| **Content Versioning & Cohort Pinning** | Реализация | 🔴 | `api` | `version`, `course_versions`, `batches.content_version`. |
| **Движок LRS (Аналитика xAPI)** | Реализация | 🔴 | `api` | Инвариантный слой TimescaleDB/ClickHouse. ADR: `2026.09.28-0002.md`. |
| **Offline-First PWA: IndexedDB Storage** | Реализация | 🟢 | `client` | `storage.rs`: `offline_xapi_statements`, `client_clock`. |
| **Offline-First PWA: Sync Algorithm** | Реализация | 🟡 | `client` | Чанки (50 шт.), Two-Phase Commit, Retry (3/10). **Синхронизация с бэкендом работает через `mock_sync_api`.** |
| **Offline-First PWA: Service Worker & Assets** | Реализация | 🟡 | `client` | Базовый скеффолд: `manifest.json`, `sw.js`, иконки, splash-экраны. |
| **Ограничения iOS PWA (деградация)** | Реализация | 🟢 | `client` / `docs` | Лимиты IndexedDB, отсутствие Background Sync задокументированы. |
| **Signed URLs для медиа** | Реализация | 🔴 | `api` / `client` | HMAC-SHA256 + TTL; см. `OPEN_API.md` §3.3. |
| **DRM (Widevine / FairPlay / PlayReady)** | Реализация | 🔴 | `api` / `client` | Опционально для Enterprise-тенантов. |
| **Рантайм плагинов Контур А (iframe)** | Реализация | 🔴 | `server` / `client` | Мост postMessage и CSP-изоляция шлюза. |
| **Рантайм плагинов Контур Б (WASM)** | Реализация | 🔴 | `server` / `client` | Zero-copy биндинги. Подпись и kill switch — ADR: `2026.09.29-0003.md`. |
| **Подпись и kill switch WASM-плагинов** | Реализация | 🔴 | `api` / `client` | Ed25519 + Revocation List. ADR: `2026.09.29-0003.md`. |
| **SBOM и сканирование уязвимостей плагинов** | Реализация | 🔴 | `api` | Генерация CycloneDX, `osv-scanner`. ADR: `2026.09.29-0010.md`. |
| **Автоматическая сертификация** | Реализация | 🔴 | `server` | Фоновые воркеры (Tokio); Calculations Engine — часть `api`. |
| **Импорт SCORM 1.2 / 2004** | Реализация | 🔴 | `api` | Два режима: конвертация / runtime. См. `STANDARDS.md` §SCORM. |
| **LTI 1.3 (Consumer + Provider)** | Реализация | 🔴 | `api` | Требования — в `STANDARDS.md`. |
| **Conformance Testing (базовый CLI)** | Реализация | 🔴 | `cli` / `server` | CLI `rust-lms-conformance` + опциональный сервис. |
| **Доступность WCAG 2.2 AA** | Реализация | 🟡 | `ui` / `client` | Базовая структура компонентов. axe-core в CI — в плане. |
| **Локализация (i18n / l10n)** | Реализация | 🟡 | `ui` / `client` | Fluent, `*_i18n` поля, RTL. Базовые файлы `messages.ftl` созданы. |
| **Retention Policies (ILM)** | Реализация | 🔴 | `server` | Cron-воркеры + таблица `retention_policies`. |
| **Application-Level Encryption (ALE)** | Реализация | 🔴 | `api` | Envelope encryption, KMS/Vault. ADR: `2026.09.29-0004.md`. |
| **Data Residency (фиксация региона)** | Реализация | 🔴 | `api` / `server` | Размещение по регионам. ADR: `2026.09.29-0005.md`. |
| **OpenTelemetry (SDK + OTLP + span attrs)** | Реализация | 🔴 | `api` / `server` / `client` | См. `DIAGNOSTICS.md` §4. |
| **Подсистема лицензирования** | Реализация | 🔴 | `api` / `server` / `cli` | Ed25519-ключ, три типа binding, soft/hard enforcement. ADR: `2026.09.29-0008.md`. |
| **Feature Flags (Global + Tenant Overrides)** | Реализация | 🔴 | `api` / `client` | PostgreSQL + LISTEN/NOTIFY + in-memory кэш. ADR: `2026.09.29-0009.md`. |
| **Чаты (личные / групповые / курс / задание)** | Реализация | 🔴 | `api` | Требования — в `COMMUNICATIONS.md`. |
| **Комментарии к контенту (ветки)** | Реализация | 🔴 | `api` | Требования — в `COMMUNICATIONS.md`. |
| **Уведомления (in-app / Email / Telegram / Webhook)** | Реализация | 🔴 | `server` | Требования — в `COMMUNICATIONS.md`. |
| **ВКС: P2P (Mesh) для 1-to-1** | Реализация | 🔴 | `client` | Требования — в `CONFERENCING.md`. |
| **ВКС: SFU (Mediasoup/Janus) для групп** | Реализация | 🔴 | `api` | Требования — в `CONFERENCING.md`. |
| **ВКС: Whiteboard, шеринг, опросы** | Реализация | 🔴 | `ui` | Требования — в `CONFERENCING.md`. |
| **ВКС: локальные TURN/STUN (Air-gapped)** | Реализация | 🔴 | `server` | Требования — в `CONFERENCING.md` и `DEPLOY.md`. |

---

## 🗺️ Плановые направления (Легенда C)

Все направления зафиксированы в `specs/ROADMAP.md`. Статус ⚪ — реализация не начата и не декомпозирована на таски.

| Направление | Статус | Примечания |
| :-- | :-: | :-- |
| Аналитика обучения (дашборды, отчёты, алерты) | ⚪ | Сбор данных покрыт LRS; слой осмысления — в плане. |
| Сертификация и валидация (Open Badges / Blockcerts) | ⚪ | Выдача и верификация сертификатов; см. `SPECIFICATION.md` §4. |
| Платежи и биллинг (подписки, счета, налоги) | ⚪ | Интеграция с провайдерами. |
| Поиск (Elasticsearch/Meilisearch) | ⚪ | Полнотекстовый поиск по контенту. |
| Рекомендации курсов и материалов | ⚪ | На основе истории и профиля. |
| Вебинары с записью | ⚪ | Поверх встроенной ВКС. |
| **Мобильное приложение (нативное)** | ⚪ | **Приоритет для iOS-сегмента** (см. `OFFLINE_SYNC.md` §1.3). Технологии будут определены позже. |
| Аудит и compliance (SIEM, retention) | ⚪ | Расширение `specs/STANDARDS.md`. |
| Conformance Testing (расширение для On-Premise) | ⚪ | Полный ADL SCORM Test Suite, cmi5, LTI 1.3, SCIM 2.0; UI в `cpanel`. |
| Расширенное a11y-тестирование (Playwright + скринридеры) | ⚪ | Для Enterprise-сертификации WCAG 2.2 AA. |
| Feature Flags: A/B-тестирование и автоматизированный rollout | ⚪ | Расширение базового механизма (см. `ROADMAP.md`). |