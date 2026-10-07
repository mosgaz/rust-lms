# Спецификация Технического Задания: Архитектура Базы Данных

**Файл спецификации:** `DB_SCHEMA.md`

> Сводная картина — в [`ARCHITECTURE.md`](ARCHITECTURE.md) §1 и §3. Регламент миграций — в [`MIGRATIONS.md`](MIGRATIONS.md). Количественные NFR (RTO/RPO, лимиты) — в [`NFR.md`](NFR.md). Правила i18n и локализации — в [`STANDARDS.md`](STANDARDS.md) §«Локализация». Feature Flags — в [`FEATURE_FLAGS.md`](FEATURE_FLAGS.md) и ADR [`2026.09.29-0009.md`](decisions/2026.09.29-0009.md). Identity-First архитектура — в ADR [`2026.10.05-0011.md`](decisions/2026.10.05-0011.md). Обоснование выбора СУБД для LRS — в ADR [`20261006-0012-lrs-storage-decision.md`](decisions/20261006-0012-lrs-storage-decision.md).

## 1. Реляционный слой (PostgreSQL Core) и Стратегия Мультитенантности

Для реализации жесткого, изолированного разделения данных между организациями (Strict Multi-tenancy) на уровне СУБД устанавливается паттерн Shared Database, Shared Schema с использованием встроенного механизма Row-Level Security (RLS).

### 1.1. Системный регламент Row-Level Security (RLS)

1. Каждая таблица, содержащая конфиденциальные, коммерческие или изолированные бизнес-данные конкретного тенанта, обязана содержать колонку `tenant_id UUID NOT NULL`.
2. Для всех изолированных таблиц при создании структуры выполняется императивная команда активации защиты: `ALTER TABLE <table_name> ENABLE ROW LEVEL SECURITY;`.
3. Создается единая системная политика фильтрации строк. СУБД автоматически сопоставляет значение в колонке `tenant_id` с идентификатором текущего тенанта, переданным из сессионного пула подключений бэкенда на Rust с помощью конфигурационного параметра сессии:

```sql
CREATE POLICY tenant_isolation_policy ON <table_name>
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
```

4. Исключение из правил RLS составляют глобальные инфраструктурные таблицы служебного уровня (`tenants`, `identities`, `tenant_api_keys`, `feature_flags`, `license`, `revoked_jwt_kids`), доступ к которым имеет исключительно супер-администратор системы или специализированный авторизационный слой шлюза безопасности.

> **Установка контекста.** Контекст тенанта устанавливается через `set_config('app.current_tenant_id', $1, true)` внутри ACID-транзакции (см. [`CODING_STANDARDS.md`](CODING_STANDARDS.md) §2.1). Прямое использование `SET LOCAL app.current_tenant_id = $1` не поддерживает параметризацию через `$1` и не применяется.

> **Границы применимости.** Принудительная гарантия RLS действует только для реляционного слоя PostgreSQL (все бизнес-таблицы, `tenant_api_keys`, `tenant_feature_flags`, Вариант А LRS — TimescaleDB). Для Варианта Б LRS (ClickHouse) изоляция обеспечивается архитектурно — см. §2 Вариант Б. Требования к тестам изоляции обязательны для обоих вариантов.

### 1.2. Сводные DDL-фрагменты глобального слоя

Ниже приведены эталонные DDL-фрагменты для глобальных таблиц `identities` и `tenants`. Полные спецификации полей — в §1.3 ниже; DDL-фрагменты служат наглядным ориентиром и должны быть синхронизированы с этими спецификациями при миграциях.

```sql
CREATE TABLE identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL, -- Хэш Argon2id в формате PHC string
    preferred_tenant_id UUID,            -- Направление на дефолтный тенант
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

> **Примечание.** В §1.3 ниже `password_hash` описан как `TEXT` (для PHC-строк произвольной длины), а `tenants` содержит расширенный набор полей (`custom_domain`, `default_locale`, `sso_config`, `branding_config`, `status`, `updated_at`). DDL-фрагменты здесь — минимальный скелет; источник истины — полевая спецификация §1.3.

### 1.3. Реляционные сущности и декларативные связи

#### Глобальная таблица: tenants (Изолирована от RLS)

Предназначена для регистрации организаций в системе и хранения их метаданных.

* `id`: UUID (Primary Key, генерируется автоматически через системную функцию `gen_random_uuid()`).
* `custom_domain`: VARCHAR(255) (Уникальный внешний веб-адрес тенанта, NULLable).
* `default_locale`: VARCHAR(16) (BCP-47: `'en'`, `'ru'`, `'ar'` и т.д. Основной язык интерфейса и метаданных тенанта).
* `sso_config`: JSONB (Декларативная конфигурация Identity Provider: OIDC Client ID, OIDC Client Secret, SAML Metadata URL, LDAP Search Base, TLS-сертификаты).
* `branding_config`: JSONB (Переменные UI-кита: цветовые гексакоды для маппинга в Tailwind CSS текущей сессии тенанта, пути к логотипам в DAM).
* `status`: VARCHAR(32) (Ограничение CHECK: `'active'`, `'suspended'`, `'archived'`).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Глобальная таблица: identities (Изолирована от RLS)

Предназначена для хранения глобальных учетных данных личности. Одна личность (один email) может иметь доступ к нескольким тенантам. Архитектура Identity-First (см. ADR [`2026.10.05-0011.md`](decisions/2026.10.05-0011.md)).

* `id`: UUID (Primary Key, `gen_random_uuid()`).
* `email`: VARCHAR(255) (Уникальный, `UNIQUE NOT NULL`). Адрес электронной почты, используемый для входа в систему.
* `password_hash`: TEXT (Хэш пароля в формате PHC string, например `$argon2id$...`, см. RFC 9106).
* `preferred_tenant_id`: UUID (FK -> tenants.id ON DELETE SET NULL, NULLable). Используется для автоматического выбора тенанта при успешной аутентификации, если тенант активен.
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** `CHECK (char_length(email) BETWEEN 3 AND 255)`, `CHECK (char_length(password_hash) BETWEEN 10 AND 1024)`.
* **Индексы:** `idx_identities_email` (для быстрого поиска при логине), `idx_identities_preferred_tenant_id`.

> **Примечание:** Эта таблица не защищена RLS, так как проверка пароля происходит до определения контекста тенанта. Доступ к ней имеет только слой аутентификации.

#### Глобальная таблица: tenant_api_keys (Изолирована от RLS)

Служит для валидации внешних систем, обращающихся к Open API тенанта.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `key_hash`: VARCHAR(64) (Хэш непрозрачного токена по алгоритму SHA-256).
* `scopes`: VARCHAR(64)[] (Массив разрешенных зон видимости для токена).
* `expires_at`: TIMESTAMPTZ (Срок действия ключа, NULLable для бессрочных токенов).
* `created_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (key_hash)`.

#### Глобальная таблица: feature_flags (Изолирована от RLS)

Глобальные функциональные флаги платформы. Определяют значение по умолчанию для всех тенантов и признак делегируемости.

* `id`: UUID (Primary Key).
* `name`: VARCHAR(64) (UNIQUE. Строковый идентификатор: `'vks'`, `'scim'`, `'ale'`, `'conformance_testing'`, …).
* `description`: TEXT (Человекочитаемое описание для админ-панели).
* `enabled`: BOOLEAN (NOT NULL, DEFAULT FALSE). Значение по умолчанию для всех тенантов.
* `is_delegatable`: BOOLEAN (NOT NULL, DEFAULT FALSE). Может ли тенант-админ переключать флаг для своего тенанта.
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (name)`.

#### Таблица: tenant_feature_flags (Защищена RLS)

Тенантные переопределения функциональных флагов. Если для пары `(tenant_id, flag_name)` записи нет — применяется значение `enabled` из глобальной таблицы `feature_flags`.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `flag_name`: VARCHAR(64) (FK -> feature_flags.name ON DELETE CASCADE).
* `enabled`: BOOLEAN (NOT NULL). Значение override.
* `updated_by`: UUID (FK -> users.id ON DELETE SET NULL, NULLable). Кто изменил (через UI/API) или NULL при изменении через CLI супер-админом.
* `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, flag_name)`.

> Механизм использования — в [`FEATURE_FLAGS.md`](FEATURE_FLAGS.md) §3. Архитектурное решение — в ADR [`2026.09.29-0009.md`](decisions/2026.09.29-0009.md). Связь с лицензированием — в [`LICENSING.md`](LICENSING.md).

#### Глобальная таблица: license (Изолирована от RLS)

Единственная запись (single-row). Хранит текущую лицензию платформы и результат последней валидации.

* `id`: UUID (Primary Key).
* `license_id`: VARCHAR(64) (UNIQUE). Идентификатор из payload лицензии.
* `customer_name`: VARCHAR(255).
* `issued_at` / `expires_at`: TIMESTAMPTZ.
* `grace_period_days`: INTEGER.
* `binding_type`: VARCHAR(32) (CHECK: `'node_locked'`, `'domain_locked'`, `'floating'`).
* `hardware_id`: VARCHAR(128) (NULLable).
* `domain_fqdn`: VARCHAR(255) (NULLable).
* `limits`: JSONB.
* `features`: JSONB (коммерческие права: `{"ale": true, "scim": true, "drm": false, ...}`).
* `raw_payload`: JSONB (полный payload для аудита).
* `signature_verified`: BOOLEAN.
* `last_validated_at`: TIMESTAMPTZ.
* `status`: VARCHAR(32) (CHECK: `'active'`, `'grace'`, `'expired'`, `'hard_limited'`, `'invalid'`).
* `created_at` / `updated_at`: TIMESTAMPTZ.

> Механизм лицензирования — в [`LICENSING.md`](LICENSING.md). Архитектурное решение — в ADR [`2026.09.29-0008.md`](decisions/2026.09.29-0008.md).

#### Глобальная таблица: revoked_jwt_kids (Изолирована от RLS)

Список отозванных идентификаторов ключей подписи JWT. Используется API-шлюзом для немедленного отклонения токенов, подписанных скомпрометированным ключом.

* `kid`: VARCHAR(64) (Primary Key). Идентификатор ключа из header JWT.
* `reason`: VARCHAR(255) (Причина отзыва: `'compromised'`, `'rotated'`, `'end_of_life'`).
* `revoked_by`: UUID (FK -> users.id, NULLable).
* `revoked_at`: TIMESTAMPTZ.
* `expires_at`: TIMESTAMPTZ (TTL равен максимальному времени жизни JWT; после истечения запись может быть удалена).

> Правила и runbook — в [`DEPLOY.md`](DEPLOY.md) §4.5 «Инцидент: компрометация ключей подписи JWT».

#### Таблица: retention_policies (Защищена RLS)

Политики жизненного цикла данных (ILM) на уровне тенанта. Определяют сроки хранения и действия по истечении для каждого класса данных.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `data_class`: VARCHAR(64) (Ограничение CHECK: `'xapi_statements'`, `'audit_log'`, `'chat_messages'`, `'etl_logs'`, `'certificates'`).
* `retention_days`: INTEGER (NOT NULL, > 0). 0 запрещено; для «хранить бессрочно» использовать `NULL` в сочетании с `action = 'keep'`.
* `action`: VARCHAR(32) (Ограничение CHECK: `'archive'`, `'anonymize'`, `'delete'`, `'keep'`).
* `enabled`: BOOLEAN (NOT NULL, DEFAULT TRUE).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, data_class)`.

#### Таблица: users (Защищена RLS)

Регистрирует связь личности (`identity`) с конкретным цифровым контуром организации (`tenant`). Одна личность может иметь несколько записей `users` в разных тенантах (например, как сотрудник в одном и как внешний эксперт в другом). Архитектура Identity-First (см. ADR [`2026.10.05-0011.md`](decisions/2026.10.05-0011.md)).

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `identity_id`: UUID (FK -> identities.id ON DELETE CASCADE). Ссылка на глобальную личность.
* `role`: VARCHAR(50) (NOT NULL). Роль личности в тенанте: `'student'`, `'teacher'`, `'admin'`, `'mentor'`, `'observer'`. Полная матрица ролей — в [`RBAC.md`](RBAC.md).
* `is_active`: BOOLEAN (NOT NULL, DEFAULT TRUE). Позволяет деактивировать доступ личности к конкретному тенанту без удаления глобальной учетной записи.
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Составной уникальный индекс `UNIQUE (tenant_id, identity_id)` (одна личность может быть добавлена в тенант только один раз).
* **Индексы:** `idx_users_tenant_id`, `idx_users_identity_id` (для быстрого поиска всех тенантов личности).
* **RLS Политика:** `user_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

> **Примечание:** В Identity-First архитектуре таблица `users` представляет роль личности в конкретном тенанте, а не саму личность. Глобальные данные (email, password_hash, preferred_tenant_id) хранятся в таблице `identities` (без RLS).

#### Таблица: programs (Защищена RLS)

Агрегирует долгосрочные образовательные треки тенанта.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `title`: VARCHAR(255) (Основной язык тенанта; денормализовано для поиска и индексов).
* `title_i18n`: JSONB (NULLable. Ключ — BCP-47: `{"ru": "…", "en": "…"}`).
* `description`: TEXT.
* `description_i18n`: JSONB (NULLable).
* `version`: INTEGER (NOT NULL, DEFAULT 1). Монотонно растёт при публикации новой версии.
* `certification_rules`: JSONB (Правила автоматического триггера выпуска сертификатов при закрытии всех дочерних элементов программы).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: courses (Защищена RLS)

Самостоятельные учебные курсы, входящие в программы или назначаемые обособленно.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `program_id`: UUID (FK -> programs.id ON DELETE SET NULL, NULLable).
* `title`: VARCHAR(255).
* `title_i18n`: JSONB (NULLable).
* `description`: TEXT.
* `description_i18n`: JSONB (NULLable).
* `version`: INTEGER (NOT NULL, DEFAULT 1). Текущая версия структуры курса.
* `course_tree`: JSONB (Иерархическая структура курса: декларативное дерево разделов, Юнитов, текстовых блоков и ссылок на Plugin ID).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: course_versions (Защищена RLS)

Снапшоты версий курса. Позволяют откатывать структуру и сохранять историю изменений для аудита. Используется механизмом «заморозки версии контента для конкретной когорты».

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `version`: INTEGER (NOT NULL). Номер версии, соответствующий `courses.version`.
* `course_tree`: JSONB (Снапшот структуры на момент публикации версии).
* `published_at`: TIMESTAMPTZ.
* `published_by`: UUID (FK -> users.id, NULLable). Ссылка на `users.id` (роль личности в тенанте, который опубликовал версию).
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, course_id, version)`.

#### Таблица: batches (Защищена RLS)

Организационные потоки студентов, проходящие обучение по фиксированному календарному графику.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `content_version`: INTEGER (NOT NULL). Версия курса, зафиксированная на момент старта когорты. Студенты этой когорты проходят именно эту версию, даже если курс обновлён.
* `title`: VARCHAR(128) (Название когорты / потока обучения).
* `timeline_config`: JSONB (Календарная сетка: жесткие даты автоматического открытия конкретных Юнитов, дедлайны Quizzes и временные слоты вебинаров).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Внешний ключ `content_version` ссылается на `course_versions.version` для того же `course_id` (проверка на уровне приложения + опционально composite FK).

#### Таблица: batch_enrollments (Защищена RLS)

Таблица связей для зачисления пользователей в конкретные потоки обучения.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `batch_id`: UUID (FK -> batches.id ON DELETE CASCADE).
* `user_id`: UUID (FK -> users.id ON DELETE CASCADE). Ссылка на `users.id` (роль личности в тенанте, зачисленного в поток).
* `status`: VARCHAR(32) (Ограничение CHECK: `'active'`, `'completed'`, `'dropped'`).
* `enrolled_at`: TIMESTAMPTZ.
* **Ограничения:** Составной уникальный индекс `UNIQUE (tenant_id, batch_id, user_id)`.

#### Таблица: course_enrollments (Защищена RLS)

Таблица связей для индивидуального зачисления пользователей на курсы (вне потоков).

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK → users.id ON DELETE CASCADE).
* `course_id`: UUID (FK → courses.id ON DELETE CASCADE).
* `status`: VARCHAR(32) (CHECK: `'active'`, `'completed'`, `'dropped'`).
* `progress`: NUMERIC(5, 4) (NOT NULL, DEFAULT 0.0000). Общий прогресс курса (0.0–1.0).
* `completed_lessons_weight`: NUMERIC(10, 4) (NOT NULL, DEFAULT 0.0000). Суммарный вес завершённых уроков (для O(1)-пересчёта прогресса).
* `completed_at`: TIMESTAMPTZ (NULLable). Когда курс был завершён.
* `enrolled_at`: TIMESTAMPTZ (NOT NULL, DEFAULT NOW()).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, user_id, course_id)`.
* **RLS Политика:** `course_enrollment_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

> **Инкрементальный пересчёт прогресса.** Поле `completed_lessons_weight` обновляется транзакционно при каждом upsert в `lesson_progress` со `status = 'completed'`. Это позволяет пересчитывать `progress` за O(1) вместо агрегатного `SUM(weight)` по всем урокам курса.

#### Таблица: lesson_progress (Защищена RLS)

Отслеживание прогресса студента по конкретному уроку.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK → users.id ON DELETE CASCADE).
* `node_id`: UUID (FK → nodes.id ON DELETE CASCADE). Узел типа `lesson`.
* `status`: VARCHAR(32) (CHECK: `'not_started'`, `'in_progress'`, `'completed'`).
* `score`: NUMERIC(5, 4) (NULLable). Балл за тест (0.0–1.0).
* `passed`: BOOLEAN (NULLable). Сдан ли тест (вычисляется сервером на основе `score >= passing_score`).
* `time_spent_seconds`: INTEGER (NOT NULL, DEFAULT 0). Общее время в уроке.
* `attempt_count`: INTEGER (NOT NULL, DEFAULT 0). Количество попыток (увеличивается только для тестов при `status = 'completed'`).
* `last_position`: INTEGER (NOT NULL, DEFAULT 0). Позиция в медиа (секунды) для возобновления.
* `completed_at`: TIMESTAMPTZ (NULLable). Когда урок завершён.
* `client_modified_at`: TIMESTAMPTZ (NULLable). Зарезервировано для синхронизации PWA (Этап 11).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:**
  * Уникальный индекс `UNIQUE (user_id, node_id)`.
  * CHECK: `score >= 0 AND score <= 1` (если не NULL).
  * CHECK: `time_spent_seconds >= 0`, `attempt_count >= 0`, `last_position >= 0`.
* **Индексы:**
  * `idx_lesson_progress_tenant_id` (для RLS).
  * `idx_lesson_progress_user_id` (быстрый поиск прогресса студента).
  * `idx_lesson_progress_node_id` (поиск по уроку).
  * `idx_lesson_progress_status` (фильтрация по статусу).
  * `idx_lesson_progress_completed` (partial index для завершённых уроков).
* **RLS Политика:** `lesson_progress_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

> **Сервер вычисляет `passed`.** Клиент присылает только `score`, сервер вычисляет `passed = (score >= passing_score)` на основе `nodes.metadata.quiz.passing_score`. Это защита от подделки результатов тестов.

> **Архивные уроки.** Уроки с `nodes.is_archived = true` исключаются из пересчёта прогресса курса. Попытка обновить прогресс архивного урока возвращает `410 Gone`.

> **Идемпотентность.** Повторный запрос с теми же данными не перезаписывает `completed_at` — сохраняется время первого завершения.

> **Триггер денормализации веса.** Поле `courses.metadata.total_weight` автоматически пересчитывается триггером `recalculate_course_total_weight()` при любых изменениях `nodes` (создание, удаление, архивация, изменение веса). Это исключает агрегатный запрос `SUM(weight)` при каждом upsert.

#### Таблица: questions (Защищена RLS)

Хранит вопросы для тестов и контрольных точек внутри курсов. Является частью Assessments Engine (Этап 10).

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK → courses.id ON DELETE CASCADE).
* `title`: VARCHAR(512) (NOT NULL). Текст или заголовок вопроса.
* `description`: TEXT (NULLable). Дополнительное описание или контекст.
* `question_type`: VARCHAR(32) (NOT NULL). CHECK: `'multiple_choice'`, `'true_false'`, `'short_answer'`, `'long_answer'`.
* `options`: JSONB (NULLable). Массив вариантов ответов для `multiple_choice` (структура: `[{"index": 0, "text": "..."}]`).
* `correct_answer`: TEXT (NULLable). Эталонный ответ для `short_answer` или `true_false`.
* `points`: INTEGER (NOT NULL, DEFAULT 1). Баллы за правильный ответ. CHECK: `points >= 0`.
* `order`: INTEGER (NOT NULL). Порядок вопроса в тесте. CHECK: `order >= 1`.
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, course_id, order)`.
* **Индексы:** `idx_questions_tenant_id`, `idx_questions_course_id`.
* **RLS Политика:** `question_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

#### Таблица: attempts (Защищена RLS)

Фиксирует факт начала и завершения попытки прохождения теста студентом.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK → users.id ON DELETE CASCADE).
* `course_id`: UUID (FK → courses.id ON DELETE CASCADE).
* `status`: VARCHAR(32) (NOT NULL). CHECK: `'in_progress'`, `'completed'`, `'timed_out'`, `'abandoned'`.
* `started_at`: TIMESTAMPTZ (NOT NULL, DEFAULT NOW()).
* `completed_at`: TIMESTAMPTZ (NULLable).
* `score`: NUMERIC(5, 4) (NULLable). Итоговый нормализованный балл (0.0–1.0). CHECK: `score >= 0.0 AND score <= 1.0`.
* `passed`: BOOLEAN (NULLable). Признак успешной сдачи.
* `time_limit_seconds`: INTEGER (NULLable). Лимит времени на прохождение. CHECK: `time_limit_seconds > 0`.
* `time_spent_seconds`: INTEGER (NOT NULL, DEFAULT 0). Фактически затраченное время. CHECK: `time_spent_seconds >= 0`.
* `attempt_number`: INTEGER (NOT NULL). Порядковый номер попытки пользователя по этому курсу. CHECK: `attempt_number >= 1`.
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Индексы:** `idx_attempts_tenant_id`, `idx_attempts_user_course` (composite: `user_id`, `course_id`), `idx_attempts_status`.
* **RLS Политика:** `attempt_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

#### Таблица: answers (Защищена RLS)

Хранит конкретные ответы студента на вопросы в рамках попытки, включая результаты автоматической проверки.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `attempt_id`: UUID (FK → attempts.id ON DELETE CASCADE).
* `question_id`: UUID (FK → questions.id ON DELETE CASCADE).
* `answer_text`: TEXT (NOT NULL). Текст ответа студента или JSON-представление выбранных вариантов.
* `is_correct`: BOOLEAN (NULLable). Результат автоматической проверки (NULL для `long_answer`, требующего ручной проверки).
* `points_earned`: INTEGER (NOT NULL, DEFAULT 0). Заработанные баллы. CHECK: `points_earned >= 0`.
* `explanation`: TEXT (NULLable). Объяснение правильного ответа (заполняется при `is_correct = false` или для справки).
* `answered_at`: TIMESTAMPTZ (NOT NULL, DEFAULT NOW()).
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, attempt_id, question_id)` (защита от дублирования ответов на один вопрос в одной попытке).
* **Индексы:** `idx_answers_attempt_id`, `idx_answers_question_id`.
* **RLS Политика:** `answer_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

#### Таблица: quizzes (Защищена RLS)

> **Примечание:** Высокоуровневая конфигурация тестов. Детальная нормализованная структура вопросов и попыток вынесена в таблицы `questions`, `attempts` и `answers` (см. выше).

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `unit_id`: VARCHAR(128) (Строковый идентификатор ноды модуля внутри общего `course_tree`).
* `title_i18n`: JSONB (NULLable).
* `version`: INTEGER (NOT NULL, DEFAULT 1).
* `pool_config`: JSONB (Массив вопросов: типы вопросов, варианты ответов, веса баллов, правила случайной выборки из категорий).
* `passing_rules`: JSONB (Количество разрешенных попыток, тайм-ليмит на прохождение, штрафные коэффициенты).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: assignments (Защищена RLS)

Практические задания, требующие экспертной проверки ментором.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `unit_id`: VARCHAR(128).
* `title_i18n`: JSONB (NULLable).
* `version`: INTEGER (NOT NULL, DEFAULT 1).
* `instruction_text`: TEXT (Подробное техническое задание на практическую работу).
* `instruction_text_i18n`: JSONB (NULLable).
* `evaluation_rubric`: JSONB (Декларативная матрица критериев и шкал ручной проверки для ментора).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: certificates (Защищена RLS)

Реестр выданных цифровых достижений по результатам обучения.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK -> users.id ON DELETE CASCADE). Ссылка на `users.id` (роль личности в тенанте, получившего сертификат).
* `target_type`: VARCHAR(32) (Ограничение CHECK: `'course'`, `'program'`, `'batch'`).
* `target_id`: UUID (Идентификатор сущности, за которую выдан документ).
* `verification_hash`: VARCHAR(64) (Уникальный публичный хэш-код для верификации сторонними системами).
* `issued_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (verification_hash)`.

### 1.4. Контроль очистки контекста тенанта в Rust (ФСТЭК Compliance)

При использовании пула соединений `sqlx::PgPool` в `crates/api` разработчики обязаны гарантировать, что контекст тенанта очищается при возврате соединения в пул. Реализация мутаций данных должна осуществляться строго в рамках явных транзакций, где значение `app.current_tenant_id` устанавливается с флагом `is_local = true`.

Запрещено использовать методы прямой записи через `SeaORM` в обход механизмов транзакционной установки RLS-параметров хоста (см. [`CODING_STANDARDS.md`](CODING_STANDARDS.md) §2.3 и §7.9). Полный перечень ограничений линтера на использование небезопасных конструкций содержится в [`FSTECK_COMPLIANCE.md`](FSTECK_COMPLIANCE.md).

---

## 2. Аналитический слой (Инвариантная спецификация LRS)

Слой Хранилища учебного опыта (Learning Record Store) спроектирован как иммутабельная система фиксации событий высокой интенсивности. В зависимости от ИТ-ландшафта On-Premise/SaaS поставки, на этапе развертывания активируется одна из двух архитектурных опций.

> **Изоляция данных в LRS.** RLS уровня PostgreSQL применима только к Варианту А (TimescaleDB). В Варианте Б (ClickHouse) изоляция обеспечивается иначе — см. §2 Вариант Б. Обе схемы обязаны проходить тесты на изоляцию тенантов: любой запрос, возвращающий данные без фильтра по `tenant_id`, считается критической ошибкой.

### Вариант А: TimescaleDB (Реляционно-временные гипертаблицы)

Вся аналитика хранится внутри расширения PostgreSQL. Создается базовая родительская таблица `xapi_statements`, которая преобразуется в гипертаблицу, автоматически разделяемую на временные чанки по колонке `timestamp` с интервалом в 7 дней.

> **Архитектурное обоснование.** Выбор TimescaleDB как рекомендованного LRS зафиксирован в ADR [`20261006-0012-lrs-storage-decision.md`](decisions/20261006-0012-lrs-storage-decision.md).

* **Структура колонок таблицы `xapi_statements`:**
  * `tenant_id`: UUID (NOT NULL, индексируется совместно с временной меткой).
  * `statement_id`: UUID (Primary Key, UNIQUE).
  * `timestamp`: TIMESTAMPTZ (NOT NULL, время фактического совершения действия пользователем на клиентском устройстве из IndexedDB).
  * `stored_at`: TIMESTAMPTZ (NOT NULL, время физического приема и записи пакета сервером хоста).
  * `course_id`: UUID (NOT NULL, индексируется).
  * `user_id`: UUID (NOT NULL, индексируется). Ссылка на `users.id` (роль личности в тенанте, совершившего действие).
  * `actor_anonymized`: BOOLEAN (NOT NULL, DEFAULT FALSE). TRUE, если `actor` в statement заменён на анонимный идентификатор по политике retention (`action = 'anonymize'`).
  * `statement_payload`: JSONB (NOT NULL, полное несжатое JSON-LD тело xAPI Statement).
* **Индексы и сегментация:** Создается композитный индекс `(tenant_id, course_id, timestamp DESC)`.
* **Политика сжатия (Compression Policy):** По истечении 14 дней с момента записи чанки гипертаблицы автоматически переводятся в колоночный формат хранения TimescaleDB со сжатием. Данные сегментируются по колонкам `tenant_id` и `course_id` и сортируются по `timestamp DESC`.
* **Идемпотентность записи:** Первичный ключ `statement_id` (UNIQUE) обеспечивает защиту от дублей на уровне СУБД. Бэкенд `api` использует `INSERT ... ON CONFLICT (statement_id) DO NOTHING`. Повторная отправка пакета из offline-очереди PWA не создаёт дублей; `statement_id` возвращается клиенту в массиве `accepted` (см. [`OPEN_API.md`](OPEN_API.md) §3.4).

> **Альтернативная схема колонок.** В ранних прототипах использовалась схема с `id UUID NOT NULL` в составе составного PK `(id, timestamp)`, а также отдельными колонками `actor_identity_id`, `verb`, `object_id`, `payload`. В актуальной версии эта схема заменена на `statement_id` + `statement_payload` + `user_id`/`course_id` (для совместимости с Identity-First). При миграциях со старых инсталляций применять `MIGRATIONS.md`.

### Вариант Б: ClickHouse (Колоночный OLAP-кластер)

Применяется при экстремальных нагрузках для быстрой обработки аналитики в реальном времени. Данные передаются бэкендом на Rust асинхронными пакетами (Bulk Insert).

> **Изоляция тенантов в ClickHouse.** ClickHouse не поддерживает нативный Row-Level Security уровня PostgreSQL. Изоляция обеспечивается на архитектурном уровне:
> 1. **Единственная точка доступа** — все запросы к ClickHouse выполняются исключительно через крейт `api`. Прямой доступ из `client`, `server` или внешних систем запрещён.
> 2. **Обязательный фильтр `tenant_id`** — на уровне слоя доступа к данным (repository layer в `api`) каждый запрос обязан содержать `WHERE tenant_id = ?`, где значение жёстко берётся из `app.current_tenant_id` сессии. Тест на отсутствие фильтра — критический, входит в CI.
> 3. **Права на уровне СУБД** — пользователь ClickHouse, под которым работает бэкенд, имеет право только на `SELECT` и `INSERT` в таблицу `xapi_statements`; доступ к другим БД и системным таблицам запрещён.
> 4. **Тесты изоляции** — обязательный интеграционный тест: запрос от тенанта A не должен возвращать ни одной строки тенанта B, даже при попытке инъекции через `course_id` или другие параметры.
> 5. **Рекомендация для коробочных поставок.** Для коробочных поставок с требованиями строгого ИБ-аудита (госсектор, крупные корпорации, обработка ПДн) **рекомендуется Вариант А (TimescaleDB + RLS)**. Вариант Б применяется только для тенантов, готовых принять архитектурную изоляцию с компенсирующими мерами (пункты 1–4). При provisioning тенанта с требованиями максимальной изоляции — по умолчанию выбирается Вариант А.
>
> Эта схема слабее принудительной RLS PostgreSQL, но она явно описана и проверяема. Приоритет для Enterprise-тенантов с требованиями максимальной изоляции — Вариант А (TimescaleDB + RLS).

* **Спецификация таблицы `xapi_statements`:**
  * **Движок таблицы (Engine):** `MergeTree` — **без дедупликации**. Иммутабельный подход: платформа сохраняет все пришедшие statements как независимые исторические факты (см. [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) §5 и [`ARCHITECTURE.md`](ARCHITECTURE.md) §3.4). Использование `ReplacingMergeTree` и любых других дедуплицирующих движков запрещено — это нарушает принцип «все попытки сохраняются».
  * **Ключ сортировки (ORDER BY):** `(tenant_id, course_id, user_id, timestamp)` (обеспечивает мгновенный поиск и построение отчетов успеваемости в рамках тенанта и конкретного курса).
  * **Схема колонок данных:**
    * `tenant_id`: UUID.
    * `statement_id`: UUID.
    * `user_id`: UUID. Ссылка на `users.id` (роль личности в тенанте, совершившего действие).
    * `course_id`: UUID.
    * `timestamp`: DateTime64(3, 'UTC') (Время действия на клиенте с точностью до миллисекунд).
    * `stored_at`: DateTime64(3, 'UTC') (Серверное время коммита).
    * `verb_uri`: LowCardinality(String) (URI глагола xAPI; словарь оптимизирует хранение повторяющихся строк).
    * `success`: Nullable(UInt8) (Флаг успешности прохождения элемента: 0, 1 или NULL, если не применимо).
    * `score_scaled`: Nullable(Float32) (Нормализованный балл от 0.0 до 1.0).
    * `actor_anonymized`: UInt8 (NOT NULL, DEFAULT 0). 1, если `actor` анонимизирован по политике retention.
    * `raw_json`: String (Полный исходный JSON-объект statement, сжатый встроенным кодеком LZ4).

#### Идемпотентность записи в ClickHouse (без дедупликации движка)

ClickHouse не поддерживает `ON CONFLICT DO NOTHING` как PostgreSQL и не имеет дедуплицирующего движка (он запрещён — см. выше). Идемпотентность обеспечивается **на уровне приложения** в `api`:

1. При получении пакета statements от клиента (см. [`OPEN_API.md`](OPEN_API.md) §3.4) крейт `api` выполняет **batch-`SELECT`** по всем `statement_id` пакета:

   ```sql
   SELECT statement_id FROM xapi_statements
   WHERE tenant_id = ? AND statement_id IN (?, ?, ...);
   ```

2. `statement_id`, уже присутствующие в LRS, исключаются из вставляемого набора.
3. `INSERT` выполняется только для новых `statement_id`.
4. Все `statement_id` из пакета (новые и уже существующие) возвращаются клиенту в массиве `accepted` — клиент очищает их из IndexedDB как подтверждённые.

**Границы применимости.** Batch-`SELECT` имеет смысл только в пределах одного тенанта (`tenant_id` жёстко берётся из `app.current_tenant_id`). Гонка двух параллельных вставок одного и того же `statement_id` возможна в пределах миллисекунд — при этом обе вставки пройдут (ClickHouse не атомарен по `statement_id`). Это осознанный трейд-офф: дубли возможны только при одновременной повторной отправке одного и того же пакета с двух разных инстансов API, что на практике исключено, потому что клиент отправляет пакет ровно один раз за цикл синхронизации (см. [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) §4.1).

**Рекомендация.** Для сценариев, где критична строгая идемпотентность даже при гонках, используется Вариант А (TimescaleDB + `ON CONFLICT DO NOTHING`). Для ClickHouse идемпотентность обеспечивается прикладным batch-`SELECT` — этого достаточно для штатного offline-сценария.

#### Мутации и retention-анонимизация в ClickHouse

Политика retention для `xapi_statements` в Варианте Б (ClickHouse) реализуется через **мутации** (`ALTER TABLE ... UPDATE ... WHERE ...`) — это **тяжёлые асинхронные операции**:

* Мутации выполняются в фоне и могут занимать часы на больших партициях.
* Во время мутации часть запросов может блокироваться.
* Прогресс виден в `system.mutations`.
* Отмена мутации ограничена.

**Рекомендации для крупных инсталляций:**

1. Выполнять анонимизацию **в окна низкой нагрузки**, мониторить прогресс через `system.mutations`.
2. Для очень больших объёмов (>1 ТБ в партиции) использовать паттерн **«холодный архив»** вместо построчной анонимизации:
   * старые партиции (например, старше 730 дней) экспортируются в отдельное S3-совместимое хранилище в неизменном виде;
   * в ClickHouse запись заменяется на tombstone (пустой `raw_json`, сохранённые `statement_id` и агрегированные поля для аналитики);
   * доступ к холодному архиву — через отдельный сервис с ограниченными правами (только чтение), с аудитом каждого обращения.
3. Для средних объёмов (до 1 ТБ) допустима построчная мутация.

Аналогичная логика применима к Варианту А (TimescaleDB), где мутации дешевле, но всё равно выполняются в фоне.
