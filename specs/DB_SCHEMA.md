# Спецификация Технического Задания: Архитектура Базы Данных

**Файл спецификации:** `DB_SCHEMA.md`

> Сводная картина — в [`ARCHITECTURE.md`](ARCHITECTURE.md) §1 и §3. Регламент миграций — в [`MIGRATIONS.md`](MIGRATIONS.md).

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

### 1.2. Реляционные сущности и декларативные связи

#### Глобальная таблица: tenants (Изолирована от RLS)

Предназначена для регистрации организаций в системе и хранения их метаданных.

* `id`: UUID (Primary Key, генерируется аппаратно через системную функцию `gen_random_uuid()`).
* `custom_domain`: VARCHAR(255) (Уникальный внешний веб-адрес тенанта, NULLable).
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

#### Таблица: users (Защищена RLS)

Регистрирует учетные записи пользователей внутри конкретного цифрового контура организации.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `external_id`: VARCHAR(255) (Уникальный символьный идентификатор пользователя, приходящий из SSO-системы тенанта при авторизации).
* `email`: VARCHAR(255) (Адрес электронной почты пользователя).
* `first_name` / `last_name`: VARCHAR(128).
* `system_role`: VARCHAR(32) (Ограничение CHECK: `'admin'`, `'instructor'`, `'mentor'`, `'learner'`, `'observer'`).
* `metadata`: JSONB (Свободная структура атрибутов профиля, заполняемая через Custom ETL Mapper из внешних файлов импорта).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Составной уникальный индекс `UNIQUE (tenant_id, external_id)` и `UNIQUE (tenant_id, email)`.

#### Таблица: programs (Защищена RLS)

Агрегирует долгосрочные образовательные треки тенанта.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `title`: VARCHAR(255).
* `description`: TEXT.
* `certification_rules`: JSONB (Правила автоматического триггера выпуска сертификатов при закрытии всех дочерних элементов программы).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: courses (Защищена RLS)

Самостоятельные учебные курсы, входящие в программы или назначаемые обособленно.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `program_id`: UUID (FK -> programs.id ON DELETE SET NULL, NULLable).
* `title`: VARCHAR(255).
* `course_tree`: JSONB (Иерархическая структура курса: декларативное дерево разделов, Юнитов, текстовых блоков и ссылок на Plugin ID).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: batches (Защищена RLS)

Организационные потоки студентов, проходящие обучение по фиксированному календарному графику.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `title`: VARCHAR(128) (Название когорты / потока обучения).
* `timeline_config`: JSONB (Календарная сетка: жесткие даты автоматического открытия конкретных Юнитов, дедлайны Quizzes и временные слоты вебинаров).
* `created_at` / `updated_at`: TIMESTAMPTZ.

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
* `pool_config`: JSONB (Массив вопросов: типы вопросов, варианты ответов, веса баллов, правила случайной выборки из категорий).
* `passing_rules`: JSONB (Количество разрешенных попыток, тайм-лимит на прохождение, штрафные коэффициенты).
* `created_at` / `updated_at`: TIMESTAMPTZ.

#### Таблица: assignments (Защищена RLS)

Практические задания, требующие экспертной проверки ментором.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK -> tenants.id ON DELETE CASCADE).
* `course_id`: UUID (FK -> courses.id ON DELETE CASCADE).
* `unit_id`: VARCHAR(128).
* `instruction_text`: TEXT (Подробное техническое задание на практическую работу).
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

### Вариант А: TimescaleDB (Реляционно-временные гипертаблицы)

Вся аналитика хранится внутри расширения PostgreSQL. Создается базовая родительская таблица `xapi_statements`, которая преобразуется в гипертаблицу, автоматически разделяемую на временные чанки по колонке `timestamp` с интервалом в 7 дней.

* **Структура колонок таблицы `xapi_statements`:**
  * `tenant_id`: UUID (NOT NULL, индексируется совместно с временной меткой).
  * `statement_id`: UUID (Primary Key).
  * `timestamp`: TIMESTAMPTZ (NOT NULL, время фактического совершения действия пользователем на клиентском устройстве из IndexedDB).
  * `stored_at`: TIMESTAMPTZ (NOT NULL, время физического приема и записи пакета сервером хоста).
  * `course_id`: UUID (NOT NULL, индексируется).
  * `user_id`: UUID (NOT NULL, индексируется).
  * `statement_payload`: JSONB (NOT NULL, полное несжатое JSON-LD тело xAPI Statement).
* **Индексы и сегментация:** Создается композитный индекс `(tenant_id, course_id, timestamp DESC)`.
* **Политика сжатия (Compression Policy):** По истечении 14 дней с момента записи чанки гипертаблицы автоматически переводятся в колоночный формат хранения TimescaleDB со сжатием. Данные сегментируются по колонкам `tenant_id` и `course_id` и сортируются по `timestamp DESC`.

### Вариант Б: ClickHouse (Колоночный OLAP-кластер)

Применяется при экстремальных нагрузках для быстрой обработки аналитики в реальном времени. Данные передаются бэкендом на Rust асинхронными пакетами (Bulk Insert).

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
    * `raw_json`: String (Полный исходный JSON-объект statement, сжатый встроенным кодеком LZ4).