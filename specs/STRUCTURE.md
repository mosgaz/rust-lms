# Спецификация Технического Задания: Структура Проекта

**Файл спецификации:** `specs/STRUCTURE.md`

> Этот файл должен на 100% соответствовать реальному содержимому `crates/`. Любое создание/удаление/переименование файла внутри `crates/` обязано быть отражено здесь немедленно (см. `AGENTS.md` §4).

## 1. Концепция Workspace-архитектуры

Проект спроектирован по принципу микро-крейтов (micro-crates) в рамках монорепозитория (Cargo workspace). Каждая папка внутри `crates/` решает изолированную задачу рантайма, предотвращая циклическое связывание и утечку нативного серверного кода в клиентский WebAssembly-бандл.

Лимит размера крейта — ≤ 1500 строк бизнес-логики (без учёта тестов); см. `AGENTS.md` §6.

## 2. Глобальная иерархия каталогов репозитория

    .
    ├── Cargo.toml                    # Глобальный манифест Cargo workspace
    ├── config.toml                   # Конфигурация сервера (хост, порт, БД)
    ├── CONTRIBUTING.md               # Стандарты коммитов и ветвления (root)
    ├── README.md                     # Главный путеводитель по репозиторию (root)
    ├── CHANGELOG.md                  # Журнал изменений (Conventional Commits)
    │
    ├── specs/                        # Директория архитектурных спецификаций и ТЗ
    │   ├── README.md                 # Разводящая страница документации
    │   ├── AGENTS.md                 # Инструкции и чек-листы для AI-агентов
    │   ├── CODING_STANDARDS.md       # Обязательные стандарты кодинга (full-stack Rust / RLS)
    │   ├── STRUCTURE.md              # Настоящий файл: архитектурная карта папок
    │   ├── SPECIFICATION.md          # Бизнес-концепция и требования к платформе
    │   ├── ARCHITECTURE.md           # Сводный ADD: RLS, Open API, LRS, плагины, ETL
    │   ├── NFR.md                    # SLA, RTO/RPO, latency, лимиты, Sizing Guide
    │   ├── DB_SCHEMA.md              # PostgreSQL (RLS), версионирование, i18n, retention, LRS, feature flags, license
    │   ├── MIGRATIONS.md             # Регламент миграций + Runbook для администратора
    │   ├── OPEN_API.md               # REST/GraphQL, SCIM 2.0, signed-url, версионирование
    │   ├── OFFLINE_SYNC.md           # Service Workers и IndexedDB для PWA, iOS-лимиты
    │   ├── PLUGIN.md                 # Рантайм плагинов (iframe / WASM), FSM, подпись, kill switch
    │   ├── PLUGIN_DEVELOPMENT_TEMPLATE.md # ТЗ для разработчиков внешних плагинов
    │   ├── DEPLOY.md                 # Docker Compose, Nginx/CSP, Air-gapped, управление ключами, runbook JWT, Chaos
    │   ├── LICENSING.md              # Офлайн-лицензирование для коробочных поставок
    │   ├── FEATURE_FLAGS.md          # Управление функциональными флагами
    │   ├── STANDARDS.md              # SCORM, xAPI, LTI, SCIM, WCAG, i18n, GDPR, Conformance Testing, Data Portability
    │   ├── COMMUNICATIONS.md         # Чаты, комментарии, уведомления
    │   ├── CONFERENCING.md           # ВКС: WebRTC P2P/SFU, локальные TURN/STUN
    │   ├── ROADMAP.md                # Плановые направления
    │   ├── STATUS.md                 # Матрица готовности фич (3 легенды)
    │   ├── RBAC.md                   # Матрица ролей и доступов
    │   ├── DIAGNOSTICS.md            # Логирование (tracing) и трейсинг (OpenTelemetry)
    │   ├── GOTCHAS.md                # Журнал технических ловушек
    │   ├── docker-compose.yml        # Манифест локального/On-Premise развёртывания
    │   └── decisions/                # Реестр архитектурных решений (ADR)
    │       ├── README.md             # Точка входа реестра ADR
    │       ├── 2026.09.28-0001.md    # RLS вместо схем-per-tenant
    │       ├── 2026.09.28-0002.md    # Иммутабельный xAPI в LRS
    │       ├── 2026.09.29-0003.md    # Подпись и kill switch для WASM-плагинов
    │       ├── 2026.09.29-0004.md    # Application-Level Encryption
    │       ├── 2026.09.29-0005.md    # Data Residency: миграция тенанта
    │       ├── 2026.09.29-0006.md    # Выбор OTel backend для SaaS
    │       ├── 2026.09.29-0007.md    # Операционный регламент deprecation API
    │       ├── 2026.09.29-0008.md    # Формат и enforcement лицензионного ключа
    │       ├── 2026.09.29-0009.md    # Архитектура Feature Flags
    │       ├── 2026.09.29-0010.md    # Supply Chain Security для WASM-плагинов
    │       └── 2026.10.05-0011.md    # Identity-First архитектура: разделение личности и роли в тенанте
    │
    └── crates/                       # Физические Rust-крейты платформы
        ├── shared/                   # Слой сетевых контрактов, структур сущностей и DTO
        ├── ui/                       # Общая библиотека Leptos UI компонентов и Tailwind CSS
        ├── icons/                    # Типизированная библиотека SVG-иконок дизайн-системы
        ├── api/                      # Ядро бизнес-логики, СУБД PostgreSQL (RLS) и LRS (без Leptos)
        ├── client/                   # Изоморфный Full-Stack веб-интерфейс, PWA и #[server] функции
        ├── server/                   # Сервер выполнения на Axum (Точка входа, Main)
        └── cli/                      # Автономная CLI-утилита для администрирования (Air-gapped)

## 3. Детализация внутренней структуры крейтов

### 3.1. Крейт: `crates/shared` (DTO & Contracts)

Абсолютно плоский крейт без привязки к СУБД или UI. Содержит структуры данных, компилируемые как под `wasm32`, так и под нативный x86_64/arm64 сервер.

- `src/models/` — базовые структуры сущностей с типобезопасными идентификаторами и строгой привязкой к `TenantId` для мультиарендности (ADR 2026.09.28-0001). Архитектура Identity-First (ADR 2026.10.05-0011):
  - `identity.rs` — `IdentityId`, `Identity` (глобальная личность: email, preferred_tenant_id).
  - `user.rs` — `UserId`, `User` (связь личности с тенантом: identity_id, tenant_id, is_active).
  - `credentials.rs` — `IdentityCredentials` (email + password_hash для аутентификации).
  - `tenant.rs` — `TenantId`, `Tenant`.
  - Заглушки для будущих сущностей: `Course`, `Program`, `Batch`, `Certificate`.
- `src/dto/` — запросы и ответы API-интерфейсов (`ImportPayload`, `SyncPackage`).
- `src/xapi/` — строгие иммутабельные типы для генерации xAPI Statements.

**Особенность:** зависимость `sqlx` является опциональной и активируется через фичу `server`, что позволяет компилировать `shared` как для сервера (с `sqlx::Type` для идентификаторов), так и для клиента (`wasm32`) без нативных зависимостей.

### 3.2. Крейт: `crates/ui` (Shared UI Library)

Изолированная дизайн-система платформы. Содержит переиспользуемые Leptos-компоненты хоста (формы, списки, кнопки, диалоги, тостеры уведомлений Fluent-локализации), не привязанные к конкретным роутам страниц. Должна компилироваться в WASM-контур.

Ответственность за соответствие WCAG 2.2 AA (см. `STANDARDS.md` §«Доступность») лежит на этом крейте: семантика, ARIA, клавиатурный фокус, контрастность компонентов. Здесь же — поддержка RTL и локализация форматов (см. `STANDARDS.md` §«Локализация»).

### 3.3. Крейт: `crates/icons` (Design System Icons)

Типизированная библиотека SVG-иконок дизайн-системы, скомпилированная для WASM. Использует `leptos` и внешнюю коллекцию иконок. Гарантирует отсутствие лишних зависимостей в клиентском бандле за счёт tree-shaking и строгой типизации компонентов.

### 3.4. Крейт: `crates/api` (Business Logic & Data Layer)

Серверное бэкенд-ядро обработки данных. **Импорт макросов Leptos сюда аппаратно запрещен.**

- `src/auth/` — модуль аутентификации (Identity-First архитектура, ADR 2026.10.05-0011):
  - `password.rs` — `PasswordHasher` на базе Argon2id (RFC 9106, PHC-формат хэшей).
  - `jwt.rs` — `JwtManager`, `JwtClaims`, `JwtConfig`, `TokenType` (Access/Refresh/Session). Токены содержат обязательный claim `tenant_id` (кроме Session) для интеграции с RLS (ADR 2026.09.28-0001).
  - `service.rs` — `AuthService` — единая точка входа для двухшагового потока аутентификации:
    - `authenticate` — проверка email/password, возврат `AuthResult::SingleTenant` (авто-выбор preferred_tenant_id) или `AuthResult::MultiTenant` (session_token + список тенантов).
    - `select_tenant` — выбор конкретного тенанта по session_token, выдача финальных токенов, обновление preferred_tenant_id.
    - `refresh`, `create_user_in_tenant`.
- `src/database/` — менеджер пула соединений SQLx и RLS-интерцептор:
  - `pool.rs` — `DatabasePool` с конфигурируемыми лимитами соединений.
  - `rls.rs` — `RlsContext` для установки сессионной переменной `app.current_tenant_id` (см. `CODING_STANDARDS.md` §2.1 и ADR 2026.09.28-0001).
  - `repositories/` — репозитории для Identity-First архитектуры и иерархии контента:
    - `identity.rs` — `IdentityRepository` (CRUD для глобальных личностей: `find_credentials_by_email`, `update_preferred_tenant`, `create_with_password`).
    - `user.rs` — `UserRepository` (CRUD для связей identity-tenant: `find_active_tenants_for_identity`, `is_user_active_in_tenant`, `create`).
    - `tenant.rs` — `TenantRepository` (CRUD для тенантов).
    - `course.rs` — `CourseRepository` (CRUD для курсов: `create`, `find_by_id`, `find_by_tenant`, `update`, `delete`, `publish_version`).
    - `node.rs` — `NodeRepository` (CRUD для узлов иерархии с ltree: `create`, `find_by_id`, `find_children`, `find_subtree`, `find_course_tree`, `update`, `move_node`, `delete`, `reorder`).
  - `entities/` — заглушка для будущих сгенерированных сущностей SeaORM (read-only типы).
- `src/http/` — HTTP-слой на базе Axum:
  - `middleware.rs` — JWT-аутентификация: извлечение Bearer-токена из заголовка `Authorization`, валидация через `JwtManager`, инъекция `IdentityId` и `TenantId` в `Request::extensions`. Refresh/Session токены отклоняются для защищённых маршрутов.
  - `handlers.rs` — REST-обработчики с унифицированным `ApiResponse<T>`:
    - Аутентификация: `login`, `select_tenant`, `refresh`.
    - Тенанты: `create_tenant`, `get_tenant` (публичные).
    - Пользователи: `create_user`, `get_user` (tenant-scoped, защищены JWT).
    - Курсы: `list_courses`, `create_course`, `get_course`, `update_course`, `delete_course`, `publish_course` (tenant-scoped, защищены JWT).
    - Узлы иерархии: `create_root_node`, `create_child_node`, `get_node`, `get_course_tree`, `get_node_subtree`, `update_node`, `move_node`, `delete_node` (tenant-scoped, защищены JWT).
  - `router.rs` — сборка Axum-роутера с разделением на публичные (`/api/v1/auth/*`, `/api/v1/tenants/*`) и защищённые JWT (`/api/v1/users/*`, `/api/v1/courses/*`, `/api/v1/nodes/*`) маршруты.
- `src/lrs/` — низкоуровневая обработка записей LRS (пакетный импорт в TimescaleDB или ClickHouse).
- `src/etl/` — потоковые чанк-парсеры кастомного импорта пользователей (Custom ETL Mapper).
- `src/scim/` — маппинг SCIM 2.0 (RFC 7643 / 7644) на внутренние сущности `users` / `batches` (см. `OPEN_API.md` §3.5).
- `src/content/` — выдача подписанных URL для медиа, валидация прав доступа (см. `OPEN_API.md` §3.6).
- `src/license/` — валидация лицензионного ключа, enforcement лимитов, чтение/запись таблицы `license` (см. `LICENSING.md`).
- `src/features/` — Feature Flags: чтение/запись `feature_flags` и `tenant_feature_flags`, in-memory кэш, LISTEN/NOTIFY-слушатель, API-контроллеры (см. `FEATURE_FLAGS.md`).
- `src/sbom/` — генерация SBOM (CycloneDX) для WASM-плагинов, сканирование уязвимостей через `osv-scanner`, интеграция с Revocation List (см. ADR `2026.09.29-0010.md`).

Миграции БД лежат в каталоге `crates/api/migrations/` (см. `MIGRATIONS.md`) и не являются модулем внутри `api`. Текущие миграции:
- `20261003000001_init_rls_and_tenants.sql` — таблицы `tenants`, `identities` (без RLS), `users` (с RLS, связь identity-tenant), политики RLS, индексы, CHECK constraints.
- `20261006000001_create_content_hierarchy.sql` — таблицы `courses` (с RLS), `nodes` (с RLS, ltree-иерархия: parent_id + path), расширение ltree, GiST/GIN индексы.

### 3.5. Крейт: `crates/client` (Isomorphic Frontend, PWA & RPC)

Единое full-stack Leptos приложение. Компилируется в PWA. Содержит сквозную сессию, роутинг, офлайн-инфраструктуру и PWA-ассеты.

- `src/shared/` — общие модули:
  - `state.rs` — глобальный реактивный контекст `AppQueueState` (provide_context/use_context).
  - `storage.rs` — типобезопасная обертка над IndexedDB (`web-sys`) для офлайн-очередей и Client Clock.
  - `services/queue_core.rs` — обобщенное DRY-ядро очереди, специализируемое под контуры.
  - `layouts/cpanel.rs` — общий layout панели управления.
- `src/student/` — личный кабинет учащегося:
  - `pages/dashboard.rs`, `pages/player.rs` — страницы студента.
  - `services/offline_queue.rs` — специализация очереди для студента (политика retry: **3 попытки**).
- `src/admin/` — панель управления (cpanel):
  - `pages/` — dashboard, features, license, instructor, mentor.
  - `services/offline_queue.rs` — специализация очереди для админа (политика retry: **10 попыток**).
- `src/website/` — публичные посадочные страницы (landing) и реестр верификации сертификатов (certificate_verify).
- `src/auth/` — сквозной SSO/RBAC слой аутентификации (login).
- `src/i18n/` — Fluent-локализация (`locales/ru`, `locales/en`).
- `src/app.rs` — корневой компонент, глобальные сигналы и слушатели событий сети (`online`/`offline`).
- `src/main.rs` — точка входа, инициализация Leptos, регистрация Service Worker и PWA-ассетов.
- `public/` — PWA-инфраструктура: `manifest.json`, `sw.js`, полный набор иконок (включая maskable) и splash-экранов для iOS/Android.
- `assets/styles/` — Tailwind CSS и цветовая палитра дизайн-системы.

### 3.6. Крейт: `crates/server` (Axum Runtime Host)

Чисто серверное нативное приложение. Единственная точка входа, содержащая функцию `fn main()`. Порт по умолчанию: **3720**.

- `src/main.rs` — точка входа:
  - Инициализация `tracing-subscriber` с `env-filter`.
  - Загрузка конфигурации из `config.toml` с переопределением через переменные окружения `RUST_LMS_*`.
  - Создание `DatabasePool` и `JwtConfig` (секрет и TTL токенов из env: `RUST_LMS_JWT_SECRET`, `RUST_LMS_JWT_ACCESS_TTL`, `RUST_LMS_JWT_REFRESH_TTL`).
  - Монтирование Axum-роутера из `api` с `TraceLayer`.
  - Запуск TCP-слушателя с graceful shutdown (SIGINT/SIGTERM).
- `src/config.rs` — `AppConfig` с загрузкой из `config.toml` и переопределением через переменные окружения с префиксом `RUST_LMS_` (разделитель `__`). Содержит `ServerConfig` (host, port) и `DatabaseConfig` (url, max/min connections).
- Считывает инфраструктурную конфигурацию `config.toml` (корень репозитория).
- Инициализирует пулы подключений `SQLx` к PostgreSQL и ClickHouse/TimescaleDB.
- Монтирует Axum-роутер, связывает его с `#[server]` RPC-эндпоинтами крейта `client`, регистрирует REST-эндпоинты Open API (`OPEN_API.md`) и запускает Tokio рантайм.
- Запускает cron-воркеры retention-политик (см. `STANDARDS.md` §«Политики удержания данных») и воркеры вебхуков.
- Выполняет первичную валидацию лицензионного ключа при старте (см. `LICENSING.md` §4).
- Поднимает выделенное LISTEN-соединение для Feature Flags (см. `FEATURE_FLAGS.md` §8.3) и лицензии (`license_changed`).
- Периодический polling флагов (60 сек) и пересканирование SBOM активных плагинов (см. ADR `2026.09.29-0010.md` §3).

### 3.7. Крейт: `crates/cli` (Administrative CLI)

Автономная бинарная утилита `rust-lms` для администрирования в изолированных контурах (Air-gapped). Не зависит от `crates/server` и не требует запуска веб-сервера.

- `src/main.rs` — точка входа с парсингом аргументов через `clap`.
- Подкоманды: `license` (установка/проверка ключа), `features` (управление флагами), `conformance` (запуск тестов соответствия стандартам).
- Работает напрямую с БД через `sqlx` или с локальными файлами конфигурации.

## 4. Направленность зависимостей и правила изоляции (Dependency Rules)

    crates/server (Axum Main Host)
        ↓
    crates/client (Leptos App: website / student / admin)
        ↓
    crates/api (Data Layer, без Leptos)
        ↓
    crates/ui (Shared UI Kit) + crates/icons (Icons)
        ↓
    crates/shared (Flat DTOs & xAPI Schemas)

1. **Запрет обратного импорта:** Крейт `shared` не знает ничего о существовании верхних слоев. Крейт `api` никогда не зависит от Leptos-приложения `client`.
2. **Изоляция WASM-контура:** Крейты `ui`, `icons` и `client` компилируются под таргет `wasm32-unknown-unknown` для работы в браузере. Им запрещено напрямую использовать нативные методы `crates/api` или `crates/server`. Вся связь между фронтенд-компонентами и бэкенд-логикой идет строго через объявления Leptos `#[server]` RPC-функций или асинхронные вызовы сетевого Open API.
3. **Идемпотентность типов:** Общие структуры в `shared` должны использовать примитивы, одинаково сериализуемые как макросами `serde` для сервера, так и `serde_wasm_bindgen` для клиента.
4. **Автономность CLI:** Крейт `cli` не зависит от `server` и `client`. Он может использовать `shared` и `api` (для доступа к БД), но не может импортировать Leptos-зависимости.
5. **Условная компиляция `shared`:** Модели с атрибутом `#[sqlx(transparent)]` (например, `TenantId`, `UserId`, `IdentityId`) доступны только при включённой фиче `server`. Клиентский код (`client`) должен использовать модели через `serde`-сериализацию без прямого доступа к `sqlx`-типам.
6. **Identity-First архитектура (ADR 2026.10.05-0011):** Глобальная таблица `identities` (email, password_hash, preferred_tenant_id) не защищена RLS, так как проверка пароля происходит до определения контекста тенанта. Таблица `users` (связь identity_id + tenant_id) защищена RLS по `tenant_id`. Одна личность может иметь несколько записей `users` в разных тенантах.