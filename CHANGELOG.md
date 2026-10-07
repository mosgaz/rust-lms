# Changelog

**Файл спецификации:** `specs/CHANGELOG.md`

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added

#### Этап 11: Certification Engine (MVP)
- **Миграция `20261011000001_create_certificates.sql`**: таблица `certificates` с поддержкой RLS (изоляция по `tenant_id`), поля `user_id`, `target_type`, `target_id`, `verification_hash` (уникальный SHA-256), `issued_at`.
- **Модели**: `Certificate` в `shared` и `api`, `CertificateResponse` для API.
- **Репозитории**: `CertificateRepository` с методами `create`, `find_by_id`, `find_by_verification_hash`, `list_by_user`.
- **Сервисы**: 
  - `CertificateService`: бизнес-логика выдачи (`issue_course_certificate`), генерации PDF "на лету" (`generate_certificate_pdf`) и проверки (`verify_certificate`).
  - `EmailService`: асинхронная отправка писем через SMTP (`lettre` + `tokio1-rustls-tls`), формирование MIME-писем с PDF-вложением, метод `new_stub()` для тестовых сред.
- **Зависимости**: добавлены `printpdf`, `qrcode`, `lettre` в workspace и крейт `api`.
- **REST API**:
  - `GET /api/v1/certificates` — список сертификатов текущего пользователя (защищённый).
  - `GET /api/v1/certificates/verify/:hash` — публичная верификация подлинности по хешу (без аутентификации).
  - `GET /api/v1/certificates/:id/download` — скачивание PDF-сертификата (генерация on-demand, stateless).
- **Интеграция**: автоматический триггер выдачи сертификата в обработчике `update_lesson_progress` при `completion_triggered = true` (с graceful degradation: ошибки email/PDF логируются, но не прерывают прогресс).
- **Интеграционные тесты** (`api/tests/certificate_integration.rs`): 7 тестов, покрывающих создание, верификацию по хешу, список пользователя, генерацию PDF (проверка заголовка `%PDF`), работу email-стаба, интеграцию сервиса и строгую RLS-изоляцию между тенантами.

#### Этап 10: Assessments Engine
- **Миграция `20261010000001_create_assessments.sql`**: таблицы `questions`, `attempts`, `answers` с RLS, CHECK-констрейнтами и индексами.
- **Модели**: `Question`, `QuestionId`, `QuestionType`, `AnswerOption`, `Attempt`, `AttemptId`, `AttemptStatus`, `Answer`, `AnswerId`, а также DTO для запросов/ответов API.
- **Репозитории**: `QuestionRepository` (CRUD вопросов), `AttemptRepository` (управление попытками, сохранение ответов, завершение).
- **Сервисы**: `ScoringEngine` с логикой сравнения ответов, подсчёта суммарного балла и определения факта сдачи (`passed`) по пороговому значению.
- **REST API**:
  - `GET/POST /api/v1/courses/:course_id/questions` — управление вопросами курса.
- **Интеграционные тесты**: сценарии создания тестов, прохождения попыток, подсчёта баллов и автоматического обновления прогресса урока при успешной сдаче.

#### Иерархия контента и Identity-First (Продолжение)
- **api**: добавлена иерархия контента на базе PostgreSQL `ltree` (Adjacency List + ltree):
  - Миграция `20261006000001_create_content_hierarchy.sql`: таблицы `courses` и `nodes` с RLS, расширение `ltree`, GiST/GIN индексы.
  - Модели `Course`, `CourseId`, `Node`, `NodeId`, `NodeType` в `shared`.
  - `CourseRepository`: CRUD + `publish_version` (инкремент версии курса).
  - `NodeRepository`: CRUD + ltree-запросы (`find_subtree`, `find_course_tree`, `move_node` с пересчётом path поддерева).
  - 14 HTTP-эндпоинтов для курсов и узлов.
  - Поддержка гибкой вложенности: `Program → Course → Chapter → Topic → Lesson`.
  - Интеграционные тесты (`hierarchy_test.rs`).
- **shared**: добавлены модели `Identity`, `IdentityId` и `IdentityCredentials` для глобального представления личности (ADR 2026.10.05-0011).
- **api**: добавлен `IdentityRepository` для операций с глобальными личностями.
- **api**: добавлен `TokenType::Session` в JWT-инфраструктуру для короткоживущих токенов выбора тенанта (TTL 5 мин).
- **api**: добавлен REST-эндпоинт `POST /api/v1/auth/select-tenant` для двухшагового потока аутентификации.
- **api**: добавлена поддержка `preferred_tenant_id` для бесшовного автоматического выбора тенанта.
- **api**: добавлен ADR `2026.10.05-0011.md`, документирующий переход на Identity-First архитектуру.
- **api**: реализован `PasswordHasher` на базе Argon2id (RFC 9106 compliant, PHC-формат).
- **api**: реализован `JwtManager`, `JwtClaims`, `JwtConfig` для генерации и валидации токенов.
- **api**: расширен `AuthService` для координации двухшаговой аутентификации.
- **api**: добавлен JWT middleware для извлечения Bearer-токена и инъекции `IdentityId` и `TenantId` в `Request::extensions`.
- **api**: добавлено 28+ unit/integration тестов, покрывающих хеширование, JWT, `AuthService` и маппинг ошибок.
- **server**: добавлена инициализация `JwtConfig` через переменные окружения (`RUST_LMS_JWT_*`).
- **server**: точка входа Axum-хоста (`main.rs`) с инициализацией `tracing-subscriber`, загрузкой конфигурации, созданием `DatabasePool` и graceful shutdown.
- **server**: модуль конфигурации (`config.rs`) с загрузкой из `config.toml` и переопределением через `RUST_LMS_*`. Порт по умолчанию: 3720.
- **api**: реализован HTTP-слой на базе Axum с разделением на публичные и tenant-scoped маршруты.
- **api**: реализованы REST-обработчики для тенантов и пользователей.
- **api**: добавлен унифицированный `ApiResponse<T>` для консистентного формата ответов.
- **api**: реализован `DatabasePool` с конфигурируемыми лимитами соединений.
- **api**: реализован `RlsContext` для установки сессионной переменной `app.current_tenant_id` (ADR 2026.09.28-0001).

#### Этап 9: Progress Tracking & Completion
- **Миграция `20261008000001_create_lesson_progress.sql`**: таблица `lesson_progress` (tenant-scoped, RLS) с полями `user_id`, `node_id`, `status`, `score`, `passed`, `time_spent_seconds`, `attempt_count`, `last_position`, `completed_at`, `client_modified_at`. CHECK-констрейнты, индексы, уникальный ключ `(user_id, node_id)`.
- **Миграция `20261009000001_add_total_weight_to_courses.sql`**: денормализация `courses.metadata.total_weight` через триггер `recalculate_course_total_weight()` для O(1)-пересчёта прогресса курса.
- **`shared/models/lesson_progress.rs`**: модели `LessonProgress`, `LessonProgressId`, `LessonStatus`, `ProgressUpdateRequest`, `ProgressResponse`, `CourseProgressSummary`, `ProgressUpdatedEvent`.
- **`shared/models/completion.rs`**: модель `CompletionCriteria` с 4 типами правил и режимами `AllOf`/`AnyOf`.
- **`api/database/repositories/lesson_progress.rs`**: `LessonProgressRepository` с методами `upsert_and_recalculate`, `get_user_course_progress`, `get_course_students_progress`, `recalculate_course_progress`, `is_instructor_or_admin`. Транзакционный пересчёт с `SELECT FOR UPDATE`.
- **`api/services/progress.rs`**: `ProgressService` — сервисный слой, координирующий репозитории.
- **HTTP-эндпоинты** (5 штук):
  - `POST /api/v1/progress` — обновить прогресс урока.
  - `GET /api/v1/progress/me` — весь прогресс текущего пользователя.
  - `GET /api/v1/progress/me/course/:course_id` — детальный прогресс по курсу.
  - `GET /api/v1/courses/:course_id/progress` — прогресс всех студентов курса (для инструктора).
  - `POST /api/v1/courses/:course_id/progress/recalculate` — принудительный пересчёт.
- **18 интеграционных тестов** (`api/tests/lesson_progress_test.rs`): базовые операции, пересчёт весов, 4 типа критериев, AllOf/AnyOf, 404/403/409/410, PATCH-семантика, серверный `passed`, архивные уроки, RLS-изоляция.
- **Хелперы** (`api/tests/common/mod.rs`): утилиты для создания тестовых данных.

### Changed

- **`api/http/handlers/mod.rs`**: добавлены `lesson_progress` handler, `certificate_service` и `identity_repo` в `AppState`.
- **`api/http/router.rs`**: зарегистрированы новые маршруты для прогресса, тестов (Assessments) и сертификатов (Certification).
- **`api/http/handlers/lesson_progress.rs`**: добавлена логика триггера `issue_course_certificate` при `result.completion_triggered == true` (с graceful degradation).
- **api**: обновлён `AppState` — добавлены поля `course_repo`, `node_repo`, `question_repo`, `attempt_repo`, `certificate_service`.
- **api**: обновлён `router.rs` — зарегистрированы новые маршруты `/api/v1/courses/*`, `/api/v1/nodes/*`, `/api/v1/attempts/*`, `/api/v1/certificates/*` с защитой JWT middleware.
- **[BREAKING CHANGE] api/db**: Переход на Identity-First архитектуру. Миграция `20261003000001_init_rls_and_tenants.sql` полностью переписана: добавлена глобальная таблица `identities` (без RLS), таблица `users` теперь хранит только связь `identity_id` + `tenant_id` (с RLS). Удалена миграция `20261005000001_add_password_hash_to_users.sql`.
- **api**: Поток аутентификации изменён на двухшаговый (`login` → `select-tenant`) с поддержкой автоматического выбора при наличии валидного `preferred_tenant_id`.
- **api**: `UserRepository` рефакторен: методы работы с паролями перенесены в `IdentityRepository`. Добавлены методы `find_active_tenants_for_identity` и `is_user_active_in_tenant`.
- **api**: Handler `create_user` теперь делегирует создание записей в `AuthService::create_user_in_tenant` для атомарного создания `identity` и связи `user`.
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

- **specs**: добавлен `CHANGELOG.md` (этот файл).
- **specs**: добавлен `AGENTS.md` с жёсткими правилами форматирования кода и документирования (`#![deny(missing_docs)]`, пути в комментариях, `//!`, `///`).
- **specs**: актуализирован `PLAN.md` (версия 4.3) — отмечены как завершённые Этапы 10 и 11.
- **specs**: актуализированы `STRUCTURE.md`, `STATUS.md`, `DB_SCHEMA.md` и `OPEN_API.md` с детальным описанием Identity-First моделей, двухшагового потока аутентификации, новых эндпоинтов и обновлённой схемы БД.
- **specs**: добавлен раздел §2.4 в `CODING_STANDARDS.md` (offline-режим `sqlx`).