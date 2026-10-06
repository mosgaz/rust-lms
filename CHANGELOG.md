# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Added

#### Этап 9: Progress Tracking & Completion
- **Миграция `20261008000001_create_lesson_progress.sql`**: таблица `lesson_progress` (tenant-scoped, RLS) с полями `user_id`, `node_id`, `status`, `score`, `passed`, `time_spent_seconds`, `attempt_count`, `last_position`, `completed_at`, `client_modified_at` (зарезервировано для Этапа 11). CHECK-констрейнты, индексы, уникальный ключ `(user_id, node_id)`.
- **Миграция `20261009000001_add_total_weight_to_courses.sql`**: денормализация `courses.metadata.total_weight` через триггер `recalculate_course_total_weight()` для O(1)-пересчёта прогресса курса.
- **`shared/models/lesson_progress.rs`**: модели `LessonProgress`, `LessonProgressId`, `LessonStatus`, `ProgressUpdateRequest`, `ProgressResponse`, `CourseProgressSummary`, `ProgressUpdatedEvent` (заготовка для LRS).
- **`shared/models/completion.rs`**: модель `CompletionCriteria` с 4 типами правил (`MinProgress`, `MinAvgQuizScore`, `RequiredNodes`, `AllLessonsCompleted`) и режимами `AllOf`/`AnyOf`.
- **`api/database/repositories/lesson_progress.rs`**: `LessonProgressRepository` с методами `upsert_and_recalculate`, `get_user_course_progress`, `get_course_students_progress`, `recalculate_course_progress`, `is_instructor_or_admin`. Транзакционный пересчёт прогресса с `SELECT FOR UPDATE`.
- **`api/services/progress.rs`**: `ProgressService` — сервисный слой, координирующий репозитории и генерирующий `ProgressUpdatedEvent`.
- **HTTP-эндпоинты** (5 штук):
  - `POST /api/v1/progress` — обновить прогресс урока (upsert с пересчётом курса).
  - `GET /api/v1/progress/me` — весь прогресс текущего пользователя.
  - `GET /api/v1/progress/me/course/:course_id` — детальный прогресс по курсу (все уроки, включая не начатые).
  - `GET /api/v1/courses/:course_id/progress` — прогресс всех студентов курса (для инструктора, с пагинацией `?limit=&offset=`).
  - `POST /api/v1/courses/:course_id/progress/recalculate` — принудительный пересчёт (роль `instructor`/`admin`).
- **18 интеграционных тестов** (`api/tests/lesson_progress_test.rs`): базовые операции, пересчёт весов, 4 типа критериев, AllOf/AnyOf, 404/403/409/410, PATCH-семантика, серверный `passed`, архивные уроки, RLS-изоляция.
- **Хелперы** (`api/tests/common/mod.rs`): `create_test_course`, `create_test_lesson`, `create_archived_lesson`, `enroll_user_to_course`, `set_course_completion_criteria`.

#### Иерархия контента и Identity-First
- **api**: добавлена иерархия контента на базе PostgreSQL `ltree` (Adjacency List + ltree):
  - Миграция `20261006000001_create_content_hierarchy.sql`: таблицы `courses` и `nodes` с RLS, расширение `ltree`, GiST/GIN индексы.
  - Модели `Course`, `CourseId`, `Node`, `NodeId`, `NodeType` в `shared`.
  - `CourseRepository`: CRUD + `publish_version` (инкремент версии курса).
  - `NodeRepository`: CRUD + ltree-запросы (`find_subtree`, `find_course_tree`, `move_node` с пересчётом path поддерева).
  - 14 HTTP-эндпоинтов для курсов и узлов: `list_courses`, `create_course`, `get_course`, `update_course`, `delete_course`, `publish_course`, `create_root_node`, `create_child_node`, `get_node`, `get_course_tree`, `get_node_subtree`, `update_node`, `move_node`, `delete_node`.
  - Поддержка гибкой вложенности: `Program → Course → Chapter → Topic → Lesson` с возможностью пропуска уровней.
  - Интеграционные тесты (`hierarchy_test.rs`): создание иерархии, ltree-запросы, перемещение узлов, RLS-изоляция.
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

- **`api/http/handlers/mod.rs`**: добавлены `lesson_progress` handler и `ProgressService` в `AppState`.
- **`api/http/router.rs`**: зарегистрированы 5 новых маршрутов для прогресса.
- **api**: обновлён `AppState` — добавлены поля `course_repo` и `node_repo` для работы с иерархией контента.
- **api**: обновлён `router.rs` — зарегистрированы новые маршруты `/api/v1/courses/*` и `/api/v1/nodes/*` с защитой JWT middleware.
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