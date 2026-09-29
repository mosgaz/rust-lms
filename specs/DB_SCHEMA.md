# Спецификация Технического Задания: Архитектура Базы Данных

**Файл спецификации:** `DB_SCHEMA.md`

> Сводная картина — в [`ARCHITECTURE.md`](ARCHITECTURE.md) §1 и §3. Регламент миграций — в [`MIGRATIONS.md`](MIGRATIONS.md). Количественные NFR (RTO/RPO, лимиты) — в [`NFR.md`](NFR.md). Правила i18n и локализации — в [`STANDARDS.md`](STANDARDS.md) §«Локализация».

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

4. Исключение из правил RLS составляют глобальные инфраструктурные таблицы служебного уровня (`tenants`, `tenant_api_keys`), доступ к которым имеет исключительно супер-администратор системы или специализированный авторизационный слой шлюза безопасности.

> **Установка контекста.** Контекст тенанта устанавливается через `set_config('app.current_tenant_id', $1, true)` внутри ACID-транзакции (см. [`CODING_STANDARDS.md`](CODING_STANDARDS.md) §2.1). Прямое использование `SET LOCAL app.current_tenant_id = $1` не поддерживает параметризацию через `$1` и не применяется.

> **Границы применимости.** Принудительная гарантия RLS действует только для реляционного слоя PostgreSQL (все бизнес-таблицы, `tenant_api_keys`, Вариант А LRS — TimescaleDB). Для Варианта Б LRS (ClickHouse) изоляция обеспечивается архитектурно — см. §2 Вариант Б. Требования к тестам изоляции обязательны для обоих вариантов.

### 1.2. Реляционные сущности и декларативные связи

#### Глобальная таблица: tenants (Изолирована от RLS)

Предназначена для регистрации организаций в системе и хранения их метаданных.

* `id`: UUID (Primary Key, генерируется автоматически через системную функцию `gen_random_uuid()`).
* `custom_domain`: VARCHAR(255) (Уникальный внешний веб-адрес тенанта, NULLable).
* `default_locale`: VARCHAR(16) (BCP-47: `'en'`, `'ru'`, `'ar'` и т.д. Основной язык интерфейса и метаданных тенанта).
* `sso_config`: JSONB (Декларативная конфигурация Identity Provider: OIDC Client ID, OIDC Client Secret, SAML Metadata URL, LDAP Search Base, TLS-сертификаты).
* `branding_config`: JSONB (Переменные UI-кита: цветовые гексакоды для маппинга в Tailwind CSS текущей сессии тенанта, пути к логотипам в DAM).
* `status`: VARCHAR(32) (Ограничение CHECK: `'active'`, `'suspended'`, `'archived'`).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Глобальная таблица: tenant_api_keys (Изолирована от RLS)

Служит для валидации внешних систем, обращающихся к Open API тенанта.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `key_hash`: VARCHAR(64) (Хэш непрозрачного токена по алгоритму SHA-256).
* `scopes`: VARCHAR(64)[] (Массив разрешенных зон видимости для токена).
* `expires_at`: TIMESTAMPTZ (Срок действия ключа, NULLable для бессрочных токенов).
* `created_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (key_hash)`.

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

Регистрирует учетные записи пользователей внутри конкретного цифрового контура организации.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `external_id`: VARCHAR(255) (Уникальный символьный идентификатор пользователя, приходящий из SSO-системы тенанта при авторизации).
* `email`: VARCHAR(255) (Адрес электронной почты пользователя).
* `first_name` / `last_name`: VARCHAR(128).
* `system_role`: VARCHAR(32) (Ограничение CHECK: `'admin'`, `'instructor'`, `'mentor'`, `'learner'`, `'observer'`).
* `locale`: VARCHAR(16) (BCP-47. Предпочтительная локаль пользователя; NULLable — наследуется от `tenants.default_locale`).
* `metadata`: JSONB (Свободная структура атрибутов профиля, заполняемая через Custom ETL Mapper из внешних файлов импорта).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Составной уникальный индекс `UNIQUE (tenant_id, external_id)` и `UNIQUE (tenant_id, email)`.

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
* `published_by`: UUID (FK -> users.id, NULLable).
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
* `user_id`: UUID (FK -> users.id ON DELETE CASCADE).
* `status`: VARCHAR(32) (Ограничение CHECK: `'active'`, `'completed'`, `'dropped'`).
* `enrolled_at`: TIMESTAMPTZ.
* **Ограничения:** Составной уникальный индекс `UNIQUE (tenant_id, batch_id, user_id)`.

#### Таблица: quizzes (Защищена RLS)

Конфигурация автоматизированных тестов для контроля знаний внутри модулей.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `unit_id`: VARCHAR(128) (Строковый идентификатор ноды модуля внутри общего `course_tree`).
* `title_i18n`: JSONB (NULLable).
* `version`: INTEGER (NOT NULL, DEFAULT 1).
* `pool_config`: JSONB (Массив вопросов: типы вопросов, варианты ответов, веса баллов, правила случайной выборки из категорий).
* `passing_rules`: JSONB (Количество разрешенных попыток, тайм-лимит на прохождение, штрафные коэффициенты).
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
* `user_id`: UUID (FK -> users.id ON DELETE CASCADE).
* `target_type`: VARCHAR(32) (Ограничение CHECK: `'course'`, `'program'`, `'batch'`).
* `target_id`: UUID (Идентификатор сущности, за которую выдан документ).
* `verification_hash`: VARCHAR(64) (Уникальный публичный хэш-код для верификации сторонними системами).
* `issued_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (verification_hash)`.

---

## 2. Аналитический слой (Инвариантная спецификация LRS)

Слой Хранилища учебного опыта (Learning Record Store) спроектирован как иммутабельная система фиксации событий высокой интенсивности. В зависимости от ИТ-ландшафта On-Premise/SaaS поставки, на этапе развертывания активируется одна из двух архитектурных опций.

> **Изоляция данных в LRS.** RLS уровня PostgreSQL применима только к Варианту А (TimescaleDB). В Варианте Б (ClickHouse) изоляция обеспечивается иначе — см. §2 Вариант Б. Обе схемы обязаны проходить тесты на изоляцию тенантов: любой запрос, возвращающий данные без фильтра по `tenant_id`, считается критической ошибкой.

### Вариант А: TimescaleDB (Реляционно-временные гипертаблицы)

Вся аналитика хранится внутри расширения PostgreSQL. Создается базовая родительская таблица `xapi_statements`, которая преобразуется в гипертаблицу, автоматически разделяемую на временные чанки по колонке `timestamp` с интервалом в 7 дней.

* **Структура колонок таблицы `xapi_statements`:**
  * `tenant_id`: UUID (NOT NULL, индексируется совместно с временной меткой).
  * `statement_id`: UUID (Primary Key).
  * `timestamp`: TIMESTAMPTZ (NOT NULL, время фактического совершения действия пользователем на клиентском устройстве из IndexedDB).
  * `stored_at`: TIMESTAMPTZ (NOT NULL, время физического приема и записи пакета сервером хоста).
  * `course_id`: UUID (NOT NULL, индексируется).
  * `user_id`: UUID (NOT NULL, индексируется).
  * `actor_anonymized`: BOOLEAN (NOT NULL, DEFAULT FALSE). TRUE, если `actor` в statement заменён на анонимный идентификатор по политике retention (`action = 'anonymize'`).
  * `statement_payload`: JSONB (NOT NULL, полное несжатое JSON-LD тело xAPI Statement).
* **Индексы и сегментация:** Создается композитный индекс `(tenant_id, course_id, timestamp DESC)`.
* **Политика сжатия (Compression Policy):** По истечении 14 дней с момента записи чанки гипертаблицы автоматически переводятся в колоночный формат хранения TimescaleDB со сжатием. Данные сегментируются по колонкам `tenant_id` и `course_id` и сортируются по `timestamp DESC`.

### Вариант Б: ClickHouse (Колоночный OLAP-кластер)

Применяется при экстремальных нагрузках для быстрой обработки аналитики в реальном времени. Данные передаются бэкендом на Rust асинхронными пакетами (Bulk Insert).

> **Изоляция тенантов в ClickHouse.** ClickHouse не поддерживает нативный Row-Level Security уровня PostgreSQL. Изоляция обеспечивается на архитектурном уровне:
> 1. **Единственная точка доступа** — все запросы к ClickHouse выполняются исключительно через крейт `api`. Прямой доступ из `client`, `server` или внешних систем запрещён.
> 2. **Обязательный фильтр `tenant_id`** — на уровне слоя доступа к данным (repository layer в `api`) каждый запрос обязан содержать `WHERE tenant_id = ?`, где значение жёстко берётся из `app.current_tenant_id` сессии. Тест на отсутствие фильтра — критический, входит в CI.
> 3. **Права на уровне СУБД** — пользователь ClickHouse, под которым работает бэкенд, имеет право только на `SELECT` и `INSERT` в таблицу `xapi_statements`; доступ к другим БД и системным таблицам запрещён.
> 4. **Тесты изоляции** — обязательный интеграционный тест: запрос от тенанта A не должен возвращать ни одной строки тенанта B, даже при попытке инъекции через `course_id` или другие параметры.
>
> Эта схема слабее принудительной RLS PostgreSQL, но она явно описана и проверяема. Приоритет для Enterprise-тенантов с требованиями максимальной изоляции — Вариант А (TimescaleDB + RLS).

* **Спецификация таблицы `xapi_statements`:**
  * **Движок таблицы (Engine):** `MergeTree` — **без дедупликации**. Иммутабельный подход: платформа сохраняет все пришедшие statements как независимые исторические факты (см. [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) §5 и [`ARCHITECTURE.md`](ARCHITECTURE.md) §3.4). Использование `ReplacingMergeTree` и любых других дедуплицирующих движков запрещено — это нарушает принцип «все попытки сохраняются».
  * **Ключ сортировки (ORDER BY):** `(tenant_id, course_id, user_id, timestamp)` (обеспечивает мгновенный поиск и построение отчетов успеваемости в рамках тенанта и конкретного курса).
  * **Схема колонок данных:**
    * `tenant_id`: UUID.
    * `statement_id`: UUID.
    * `user_id`: UUID.
    * `course_id`: UUID.
    * `timestamp`: DateTime64(3, 'UTC') (Время действия на клиенте с точностью до миллисекунд).
    * `stored_at`: DateTime64(3, 'UTC') (Серверное время коммита).
    * `verb_uri`: LowCardinality(String) (URI глагола xAPI; словарь оптимизирует хранение повторяющихся строк).
    * `success`: Nullable(UInt8) (Флаг успешности прохождения элемента: 0, 1 или NULL, если не применимо).
    * `score_scaled`: Nullable(Float32) (Нормализованный балл от 0.0 до 1.0).
    * `actor_anonymized`: UInt8 (NOT NULL, DEFAULT 0). 1, если `actor` анонимизирован по политике retention.
    * `raw_json`: String (Полный исходный JSON-объект statement, сжатый встроенным кодеком LZ4).