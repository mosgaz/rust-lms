# Спецификация Open API: Контракты REST/GraphQL

**Файл спецификации:** `specs/OPEN_API.md`

> Смежные разделы: NFR и лимиты — в `NFR.md`; схема БД (Identity-First) — в `DB_SCHEMA.md`; офлайн-синхронизация — в `OFFLINE_SYNC.md`; архитектурные решения по аутентификации — в ADR `2026.10.05-0011.md`.

## 1. Общие принципы API

### 1.1. Версионирование
Все эндпоинты используют префикс `/api/v1/`. Изменение контракта (breaking change) требует выпуска новой версии (`/api/v2/`) и следует регламенту deprecation (ADR `2026.09.29-0007.md`).

### 1.2. Аутентификация и авторизация (Identity-First Flow)
Система использует архитектуру Identity-First: одна глобальная личность (`identity`) может иметь доступ к нескольким тенантам. Аутентификация реализована как двухшаговый процесс:

1. **Шаг 1: Идентификация (`POST /api/v1/auth/login`)**
   - Клиент отправляет `email` и `password`.
   - Сервер проверяет хэш пароля глобально (таблица `identities`).
   - Если у личности есть активный `preferred_tenant_id`, сервер немедленно возвращает финальную пару токенов (`access_token` + `refresh_token`).
   - Если `preferred_tenant_id` отсутствует, неактивен или их несколько, сервер возвращает короткий `session_token` (TTL 5 мин, без `tenant_id` в claims) и список `available_tenants` для выбора.

2. **Шаг 2: Выбор контекста (`POST /api/v1/auth/select-tenant`)**
   - Клиент отправляет `session_token` и выбранный `tenant_id`.
   - Сервер проверяет, что личность действительно имеет активный доступ к этому `tenant_id` (таблица `users`).
   - Сервер выдает финальную пару токенов (`access_token` содержит claim `tenant_id` для RLS) и обновляет `preferred_tenant_id` для будущих входов.

- **Формат токенов:** JWT (HS256). `access_token` живет 15 минут, `refresh_token` — 7 дней, `session_token` — 5 минут.
- **Передача токена:** Заголовок `Authorization: Bearer <access_token>` для всех защищённых эндпоинтов.
- **RLS-интеграция:** Middleware извлекает `tenant_id` из claims `access_token` и устанавливает сессионную переменную `app.current_tenant_id` перед выполнением любых запросов к БД.

### 1.3. Формат ответов
Все ответы возвращают JSON с полями:
- `success` — boolean, флаг успешности операции.
- `data` — полезная нагрузка (для успешных операций, `null` при ошибке).
- `error` — строка с сообщением об ошибке (для ошибок, `null` при успехе).

### 1.4. Защита от перегрузок (Rate Limiting)
Ограничение частоты входящих запросов реализуется на уровне API-шлюза бэкенда по алгоритму **Token Bucket**. Лимиты изолированы для каждого тенанта и дифференцируются в зависимости от уровня API-ключа.

Отдельные лимиты выделяются для:
- SCIM bulk-операций.
- Выдачи подписанных URL.
- Offline-синхронизации (`POST /api/v1/analytics/lrs/sync`).
- Provisioning тенантов.

---

## 2. Управление инфраструктурой (Глобальный API)

### 2.1. Provisioning тенантов

#### `POST /api/v1/internal/tenants`
- **Уровень доступа:** Глобальный токен (`infrastructure:provisioning`).
- **Назначение:** Динамическое создание нового тенанта.
- **Request:**
```json
{
  "name": "ООО «Пример»",
  "custom_domain": "client1.lms.example",
  "default_locale": "ru",
  "admin_email": "admin@example.com",
  "features": ["scim", "ale"]
}
```
- **Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "id": "uuid-v4",
    "slug": "client1",
    "name": "ООО «Пример»",
    "is_active": true
  },
  "error": null
}
```

---

## 3. Аутентификация и управление пользователями

### 3.1. Аутентификация и выбор тенанта

#### `POST /api/v1/auth/login`
- **Уровень доступа:** Публичный.
- **Назначение:** Проверка учётных данных и начало сессии.
- **Request:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```
- **Response (Авто-выбор, 200 OK):** Возвращается, если `preferred_tenant_id` установлен и активен.
```json
{
  "success": true,
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer"
  },
  "error": null
}
```
- **Response (Требуется выбор, 200 OK):** Возвращается, если тенантов несколько или `preferred_tenant_id` невалиден.
```json
{
  "success": true,
  "data": {
    "session_token": "eyJ...",
    "available_tenants": [
      { "id": "uuid-1", "name": "ООО Ромашка", "slug": "romashka" },
      { "id": "uuid-2", "name": "ИП Иванов", "slug": "ivanov" }
    ]
  },
  "error": null
}
```

#### `POST /api/v1/auth/select-tenant`
- **Уровень доступа:** Требуется валидный `session_token` (передается в теле запроса).
- **Назначение:** Выбор конкретного тенанта из списка доступных и получение финальных токенов.
- **Request:**
```json
{
  "session_token": "eyJ...",
  "tenant_id": "uuid-1"
}
```
- **Response (200 OK):** Возвращает финальные токены. Сервер также обновляет `preferred_tenant_id` для этой личности.
```json
{
  "success": true,
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "token_type": "Bearer"
  },
  "error": null
}
```

#### `POST /api/v1/auth/refresh`
- **Уровень доступа:** Требуется валидный `refresh_token` (передается в теле запроса).
- **Назначение:** Получение новой пары токенов без повторного ввода пароля.
- **Request:**
```json
{
  "refresh_token": "eyJ..."
}
```
- **Response (200 OK):** Аналогичен успешному ответу `login` (авто-выбор).

### 3.2. Кастомный импорт пользователей (ETL)

#### `POST /api/v1/users/import-custom`
- **Уровень доступа:** Локальный токен тенанта (`access_token`) со scope `users:sync`.
- **Назначение:** Массовый импорт пользователей из CSV/XLSX/JSON. Создает новые `identities` или связывает существующие с текущим `tenant_id`.
- **Request:** `multipart/form-data` с файлом и маппингом полей.
- **Response (202 Accepted):**
```json
{
  "success": true,
  "data": {
    "job_id": "uuid-v4",
    "status": "queued",
    "estimated_duration_sec": 120
  },
  "error": null
}
```

### 3.3. SCIM 2.0 (Real-time Provisioning)

#### `GET /scim/v2/Users`
#### `POST /scim/v2/Users`
#### `PATCH /scim/v2/Users/{id}`
#### `DELETE /scim/v2/Users/{id}`
- **Уровень доступа:** Локальный токен тенанта со scope `scim:sync`.
- **Назначение:** Автоматическая синхронизация жизненного цикла пользователей с внешними HRIS/HRM.
- **Контракт:** RFC 7643 / 7644.
- **Idempotency:** Все POST-запросы обязаны содержать `X-Idempotency-Key`.

### 3.4. Выдача подписанных URL для медиа

#### `POST /api/v1/content/signed-url`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Генерация временного URL для доступа к медиафайлу в DAM.
- **Request:**
```json
{
  "file_id": "uuid-v4",
  "ttl_sec": 3600
}
```
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "signed_url": "https://dam.example.com/files/uuid?signature=...",
    "expires_at": "2026-10-02T15:30:00Z"
  },
  "error": null
}
```
- **Алгоритм подписи:** HMAC-SHA256 с коротким TTL.

### 3.5. Офлайн-синхронизация xAPI-стейтментов

#### `POST /api/v1/analytics/lrs/sync`
- **Уровень доступа:** Локальный токен тенанта со scope `analytics:write`.
- **Назначение:** Прием пакета xAPI-стейтментов от клиента (PWA) для сохранения в LRS.
- **Контекст:** Используется клиентом при восстановлении сети для синхронизации офлайн-очереди IndexedDB (см. `OFFLINE_SYNC.md` §4).

**Request:**
```json
{
  "statements": [
    {
      "id": "uuid-v4",
      "timestamp": "2026-10-02T14:30:00Z",
      "actor": {
        "objectType": "Agent",
        "account": {
          "homePage": "https://tenant.lms.example",
          "name": "user_123"
        }
      },
      "verb": {
        "id": "http://adlnet.gov/expapi/verbs/completed",
        "display": { "en": "completed" }
      },
      "object": {
        "objectType": "Activity",
        "id": "https://tenant.lms.example/courses/uuid/units/uuid"
      }
    }
  ]
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "accepted": ["uuid-v4-1", "uuid-v4-2"],
    "rejected": [
      {
        "id": "uuid-v4-3",
        "reason": "duplicate",
        "message": "Statement with this ID already exists in LRS"
      }
    ]
  },
  "error": null
}
```

**Поведение сервера:**
1. Валидация схемы каждого стейтмента (xAPI JSON-LD, см. `STANDARDS.md` §«xAPI»).
2. Проверка на дубликаты по полю `id` (идемпотентность).
3. Сохранение уникальных стейтментов в LRS (TimescaleDB или ClickHouse, см. `DB_SCHEMA.md` §2).
4. Возврат массивов `accepted` (успешно сохраненные ID) и `rejected` (ID + причина отклонения).

**NFR-ссылки:**
- Лимит на размер запроса: **≤ 5 МБ** (см. `NFR.md` §4).
- Лимит на количество стейтментов в пакете: **≤ 100**.
- Пропускная способность: **500 пакетов/сек** на инстанс (см. `NFR.md` §2).
- Latency: p95 ≤ 600 мс (см. `NFR.md` §3).

**Клиентская логика (Two-Phase Commit):**
- Клиент отправляет пакет через `POST /api/v1/analytics/lrs/sync`.
- При получении `200 OK` с массивом `accepted` — атомарное удаление этих стейтментов из IndexedDB (см. `OFFLINE_SYNC.md` §4.1).
- При ошибке сети или сервера — инкремент `retry_count` и повторная попытка при следующем `online` event.

---

## 4. Управление обучением

### 4.1. Курсы и программы

#### `GET /api/v1/courses`
#### `POST /api/v1/courses`
#### `PATCH /api/v1/courses/{id}`
#### `DELETE /api/v1/courses/{id}`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Управление курсами внутри тенанта.

### 4.2. Версионирование контента

#### `POST /api/v1/courses/{id}/publish`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Публикация новой версии курса (инкремент `version` в `courses`, см. `DB_SCHEMA.md` §1.3).
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "course_id": "uuid-v4",
    "new_version": 3,
    "published_at": "2026-10-02T14:30:00Z"
  },
  "error": null
}
```

### 4.3. Иерархия контента (Courses & Nodes)

Иерархия контента построена на единой таблице `nodes` с использованием PostgreSQL-расширения `ltree` (Adjacency List + ltree). Поддерживается гибкая вложенность: `Program → Course → Chapter → Topic → Lesson`, с возможностью пропуска промежуточных уровней.

> **См. также:** `DB_SCHEMA.md` §1.3 (описание таблиц `courses` и `nodes`), ADR по Identity-First архитектуре.

#### `GET /api/v1/courses`
- **Уровень доступа:** Локальный токен тенанта (JWT Bearer).
- **Назначение:** Получение списка курсов текущего тенанта (пагинация: limit 100, offset 0).
- **Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid-v4",
      "tenant_id": "uuid-v4",
      "title": "Введение в Rust",
      "description": "Базовый курс для начинающих",
      "version": 1
    }
  ],
  "error": null
}
```

#### `POST /api/v1/courses`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Создание нового курса.
- **Request:**
```json
{
  "title": "Введение в Rust",
  "title_i18n": {"en": "Introduction to Rust", "ru": "Введение в Rust"},
  "description": "Базовый курс",
  "description_i18n": null,
  "certification_rules": {"auto_issue": true}
}
```
- **Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "id": "uuid-v4",
    "tenant_id": "uuid-v4",
    "title": "Введение в Rust",
    "version": 1
  },
  "error": null
}
```

#### `GET /api/v1/courses/:id`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение метаданных курса по идентификатору.
- **Response (200 OK):** Аналогично `POST /api/v1/courses`.
- **Ошибки:** `404 Not Found` (курс не существует или принадлежит другому тенанту — RLS).

#### `PATCH /api/v1/courses/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Частичное обновление метаданных курса.
- **Request:** (любое подмножество полей)
```json
{
  "title": "Новое название курса",
  "description": "Обновлённое описание"
}
```

#### `DELETE /api/v1/courses/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Удаление курса. **Каскадно удаляет все связанные узлы** (через FK `ON DELETE CASCADE`).
- **Response (204 No Content).**

#### `POST /api/v1/courses/:id/publish`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Публикация новой версии курса (инкремент `version`).
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "course_id": "uuid-v4",
    "new_version": 2
  },
  "error": null
}
```

#### `GET /api/v1/courses/:id/tree`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение полного дерева узлов курса (все главы, темы, уроки), отсортированное по `path` (ltree) и `sort_order`.
- **Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid-chapter",
      "parent_id": null,
      "node_type": "chapter",
      "course_id": "uuid-course",
      "title": "Глава 1: Основы",
      "metadata": {},
      "sort_order": 0
    },
    {
      "id": "uuid-lesson",
      "parent_id": "uuid-chapter",
      "node_type": "lesson",
      "course_id": "uuid-course",
      "title": "Урок 1.1: Синтаксис",
      "metadata": {"content_type": "video", "video": {"file_id": "uuid-dam"}},
      "sort_order": 0
    }
  ],
  "error": null
}
```

#### `POST /api/v1/courses/:id/nodes`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Создание корневого узла курса (глава или изолированный урок). Автоматически устанавливает `course_id` и генерирует `path` на основе `node_id`.
- **Request:**
```json
{
  "node_type": "chapter",
  "title": "Глава 1",
  "title_i18n": null,
  "description": "Основы языка",
  "metadata": {}
}
```
- **Response (201 Created):** Объект узла.
- **Ошибки:** `404 Not Found` (курс не существует).

#### `POST /api/v1/nodes/:id/children`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Создание дочернего узла. Автоматически наследует `course_id` от родителя и вычисляет `path` как `parent_path.node_id`.
- **Request:** Аналогично `POST /api/v1/courses/:id/nodes`.
- **Response (201 Created):** Объект узла.
- **Ошибки:** `404 Not Found` (родительский узел не существует).

#### `GET /api/v1/nodes/:id`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение метаданных отдельного узла.

#### `GET /api/v1/nodes/:id/subtree`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение поддерева, начиная с указанного узла (через ltree-оператор `<@`). Возвращает сам узел и всех его потомков.
- **Response (200 OK):** Массив узлов, отсортированный по `path` и `sort_order`.

#### `PATCH /api/v1/nodes/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Обновление полей узла (`title`, `title_i18n`, `description`, `metadata`). **Не изменяет структуру дерева.**
- **Request:**
```json
{
  "title": "Новое название",
  "metadata": {"content_type": "video", "video": {"file_id": "new-uuid"}}
}
```

#### `POST /api/v1/nodes/:id/move`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Перемещение узла под нового родителя. **Автоматически пересчитывает `path` для всего поддерева** (через `text2ltree` и ltree-операторы).
- **Request:**
```json
{
  "new_parent_id": "uuid-new-parent"
}
```
- **Response (200 OK):** Обновлённый объект узла с новым `parent_id`.
- **Ошибки:** `404 Not Found` (узел или новый родитель не существуют).
- **Примечание:** Для перемещения в корень (сделать узел корневым) передайте `"new_parent_id": null`.

#### `DELETE /api/v1/nodes/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Удаление узла. **Каскадно удаляет всё поддерево** (через FK `ON DELETE CASCADE` на `parent_id`).
- **Response (204 No Content).**

### 4.4. Потоки и зачисления (Batches & Enrollments)

Потоки (Batches) — это группы студентов, проходящих курсы вместе в определённые сроки. Поддерживаются 4 роли участников: `student`, `instructor`, `tutor`, `observer`. Индивидуальные зачисления на курсы (self-paced) реализованы через отдельную таблицу `course_enrollments`.

> **См. также:** `DB_SCHEMA.md` §1.3 (описание таблиц `batches`, `batch_courses`, `batch_enrollments`, `course_enrollments`).

#### `GET /api/v1/batches`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение списка потоков текущего тенанта (пагинация: limit 100, offset 0).
- **Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid-v4",
      "tenant_id": "uuid-v4",
      "title": "Осенний поток 2026",
      "status": "active",
      "start_date": "2026-09-01T00:00:00Z",
      "end_date": "2026-12-31T23:59:59Z"
    }
  ],
  "error": null
}
```

#### `POST /api/v1/batches`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Создание нового потока.
- **Request:**
```json
{
  "title": "Осенний поток 2026",
  "description": "Корпоративное обучение",
  "status": "draft",
  "start_date": "2026-09-01T00:00:00Z",
  "end_date": "2026-12-31T23:59:59Z",
  "enrollment_deadline": "2026-08-15T23:59:59Z"
}
```
- **Response (201 Created):** Объект потока.

#### `GET /api/v1/batches/:id`
- **Уровень доступа:** Локальный токен тенанта.
- **Ошибки:** `404 Not Found` (поток не существует или принадлежит другому тенанту — RLS).

#### `PATCH /api/v1/batches/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Частичное обновление метаданных потока.

#### `DELETE /api/v1/batches/:id`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Удаление потока. **Каскадно удаляет все связанные `batch_courses` и `batch_enrollments`**.
- **Response (204 No Content).**

#### `POST /api/v1/batches/:id/enroll`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Зачисление пользователя в поток с указанной ролью.
- **Request:**
```json
{
  "user_id": "uuid-v4",
  "role": "student"
}
```
- **Response (201 Created):** Объект зачисления.
- **Ошибки:** 
  - `404 Not Found` (поток не существует)
  - `409 Conflict` (пользователь уже зачислен в поток)

#### `DELETE /api/v1/batches/:batch_id/enroll/:user_id`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Отчисление пользователя из потока (**soft delete**: статус меняется на `dropped`).
- **Response (204 No Content).**

#### `PATCH /api/v1/batches/:batch_id/enroll/:user_id`
- **Уровень доступа:** Локальный токен тенанта со scope `batches:write`.
- **Назначение:** Изменение роли участника потока.
- **Request:**
```json
{
  "role": "instructor"
}
```

#### `GET /api/v1/batches/:id/enrollments`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение списка участников потока (студенты, инструкторы, тьюторы, наблюдатели).

#### `POST /api/v1/courses/:id/enroll`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Индивидуальное зачисление пользователя на курс (self-paced).
- **Request:**
```json
{
  "user_id": "uuid-v4"
}
```
- **Response (201 Created):** Объект зачисления с `progress = 0.0`, `status = "active"`.
- **Ошибки:** 
  - `404 Not Found` (курс не существует)
  - `409 Conflict` (пользователь уже зачислен)

#### `DELETE /api/v1/courses/:course_id/enroll/:user_id`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Отчисление пользователя с курса (soft delete).
- **Response (204 No Content).**

#### `GET /api/v1/courses/:id/enrollments`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение списка студентов курса.

#### `GET /api/v1/users/:id/enrollments`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение всех курсов, на которые зачислен пользователь.

### 4.5. Прогресс обучения (Lesson Progress)

Эндпоинты для отслеживания прогресса студентов по урокам и курсам. Все эндпоинты tenant-scoped, защищены JWT middleware.

#### `POST /api/v1/progress`

Обновляет прогресс урока для текущего пользователя. Автоматически пересчитывает прогресс курса и проверяет критерии завершения.

**Уровень доступа:** Tenant-scoped токен (студент, зачисленный в курс).

**Тело запроса:**
```json
{
  "node_id": "uuid",
  "status": "completed",        // опционально: not_started | in_progress | completed
  "score": 0.85,                // опционально: 0.0–1.0 (для тестов)
  "time_spent_seconds": 1200,   // опционально
  "last_position": 300,         // опционально: позиция в медиа (секунды)
  "client_modified_at": "2026-10-07T10:00:00Z"  // опционально, для PWA-sync (Этап 11)
}
```

**Успешный ответ (200 OK):**
```json
{
  "success": true,
  "data": {
    "lesson_progress": {
      "id": "uuid",
      "user_id": "uuid",
      "node_id": "uuid",
      "status": "completed",
      "score": 0.85,
      "passed": true,
      "time_spent_seconds": 1200,
      "attempt_count": 1,
      "last_position": 300,
      "completed_at": "2026-10-07T10:00:00Z",
      "client_modified_at": "2026-10-07T10:00:00Z"
    },
    "course_progress": 0.75,
    "course_status": "in_progress",
    "completion_triggered": false
  }
}
```

**Ошибки:**
- `403 Forbidden` — пользователь не зачислен в курс (`NotEnrolled`).
- `404 Not Found` — урок не найден (`NodeNotFound`).
- `409 Conflict` — курс уже завершён (`CourseAlreadyCompleted`).
- `410 Gone` — урок архивирован (`NodeArchived`).
- `422 Unprocessable Entity` — невалидные данные (например, `score > 1.0`).

**Архитектурные решения:**
- **PATCH-семантика:** Все поля опциональны, кроме `node_id`.
- **Сервер вычисляет `passed`:** Клиентский `passed` игнорируется.
- **Идемпотентность:** Повторный запрос с теми же данными не перезаписывает `completed_at`.
- **Транзакционность:** `SELECT FOR UPDATE` на `course_enrollments` защищает от race conditions.

---

#### `GET /api/v1/progress/me`

Возвращает весь прогресс текущего пользователя по всем курсам.

**Уровень доступа:** Tenant-scoped токен (студент).

**Успешный ответ (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "course_id": "uuid",
      "progress": 0.75,
      "status": "in_progress",
      "completed_at": null,
      "completed_lessons_count": 3,
      "total_lessons_count": 4,
      "completed_lessons_weight": 3.0,
      "total_lessons_weight": 4.0
    }
  ]
}
```

---

#### `GET /api/v1/progress/me/course/:course_id`

Возвращает детальный прогресс по курсу — **все уроки**, включая не начатые.

**Уровень доступа:** Tenant-scoped токен (студент, зачисленный в курс).

**Успешный ответ (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "node_id": "uuid",
      "status": "completed",
      "score": 0.85,
      "passed": true,
      "time_spent_seconds": 1200,
      "attempt_count": 1,
      "last_position": 0,
      "completed_at": "2026-10-07T10:00:00Z"
    },
    {
      "node_id": "uuid",
      "status": "not_started",
      "score": null,
      "passed": null,
      "time_spent_seconds": 0,
      "attempt_count": 0,
      "last_position": 0,
      "completed_at": null
    }
  ]
}
```

**Ошибки:** `403 Forbidden` (не зачислен), `404 Not Found` (курс не найден).

---

#### `GET /api/v1/courses/:course_id/progress`

Прогресс всех студентов курса (для инструктора/администратора).

**Уровень доступа:** Tenant-scoped токен (роль `instructor` или `admin`).

**Параметры запроса (Query String):**
- `limit` (integer, optional, default: 100, max: 1000) — максимальное количество записей.
- `offset` (integer, optional, default: 0) — смещение от начала списка.

**Успешный ответ (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "user_id": "uuid",
      "progress": 0.75,
      "status": "in_progress",
      "completed_at": null,
      "completed_lessons_count": 3,
      "total_lessons_count": 4,
      "completed_lessons_weight": 3.0,
      "total_lessons_weight": 4.0
    }
  ]
}
```

**Ошибки:** `403 Forbidden` (недостаточно прав).

---

#### `POST /api/v1/courses/:course_id/progress/recalculate`

Принудительный пересчёт прогресса для всех зачисленных студентов курса. Полезно после массовых изменений в структуре курса (добавление/удаление уроков, изменение весов).

**Уровень доступа:** Tenant-scoped токен (роль `instructor` или `admin`).

**Успешный ответ (200 OK):**
```json
{
  "success": true,
  "data": {
    "rows_affected": 42
  }
}
```

> ⚠️ **Предупреждение:** Эта операция использует `FOR UPDATE OF ce` и блокирует все зачисления курса на время выполнения. Для больших курсов (1000+ студентов) может занять несколько секунд. Рекомендуется запускать в нерабочее время или через background worker.

**Ошибки:**
- `403 Forbidden` — недостаточно прав (требуется `instructor` или `admin`).
- `404 Not Found` — курс не найден.

### 4.6. Тестирование и оценка (Assessments Engine)

Эндпоинты для управления вопросами, прохождения тестов и автоматического подсчёта баллов. Интегрированы с прогрессом обучения (Этап 10).

> **См. также:** `DB_SCHEMA.md` §1.3 (таблицы `questions`, `attempts`, `answers`).

#### `GET /api/v1/courses/:course_id/questions`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Получение списка всех вопросов, привязанных к курсу.
- **Response (200 OK):** Массив объектов вопросов.

#### `POST /api/v1/courses/:course_id/questions`
- **Уровень доступа:** Локальный токен тенанта со scope `courses:write`.
- **Назначение:** Создание нового вопроса для курса.
- **Request:**
```json
{
  "title": "Какой язык компилируется в WebAssembly?",
  "description": "Выберите один вариант",
  "question_type": "multiple_choice",
  "options": [
    {"index": 0, "text": "Python"},
    {"index": 1, "text": "Rust"},
    {"index": 2, "text": "Ruby"}
  ],
  "correct_answer": "1",
  "points": 10,
  "order": 1
}
```
- **Response (201 Created):** Созданный объект вопроса.
- **Ошибки:** `404 Not Found` (курс не существует).

#### `POST /api/v1/courses/:course_id/attempts`
- **Уровень доступа:** Локальный токен тенанта (студент).
- **Назначение:** Начало новой попытки прохождения теста. Автоматически определяет `attempt_number`.
- **Request:**
```json
{
  "time_limit_seconds": 600
}
```
- **Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "attempt": {
      "id": "uuid-v4",
      "status": "in_progress",
      "attempt_number": 1,
      "started_at": "2026-10-07T10:00:00Z"
    },
    "questions": [ ... ]
  },
  "error": null
}
```
- **Ошибки:** `409 Conflict` (превышен лимит в 10 попыток или уже есть активная попытка).

#### `POST /api/v1/attempts/:attempt_id/answers`
- **Уровень доступа:** Локальный токен тенанта (студент).
- **Назначение:** Инкрементальное сохранение ответа на конкретный вопрос в рамках активной попытки.
- **Request:**
```json
{
  "question_id": "uuid-v4",
  "answer_text": "Rust"
}
```
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "is_correct": true,
    "points_earned": 10
  },
  "error": null
}
```

#### `POST /api/v1/attempts/:attempt_id/nodes/:node_id/complete`
- **Уровень доступа:** Локальный токен тенанта (студент).
- **Назначение:** Завершение попытки. Сервер самостоятельно подсчитывает итоговый балл (0.0–1.0), определяет факт сдачи (`passed`) и, при успехе, автоматически обновляет прогресс урока (`lesson_progress`) на статус `completed`.
- **Request:**
```json
{
  "answers": [
    {
      "question_id": "uuid-v4-1",
      "answer_text": "Rust"
    },
    {
      "question_id": "uuid-v4-2",
      "answer_text": "false"
    }
  ]
}
```
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "id": "uuid-v4",
    "status": "completed",
    "score": 1.0,
    "passed": true,
    "time_spent_seconds": 120,
    "completed_at": "2026-10-07T10:02:00Z"
  },
  "error": null
}
```
- **Ошибки:** `404 Not Found` (попытка не найдена), `409 Conflict` (попытка уже завершена или истек лимит времени).

#### `GET /api/v1/attempts/:attempt_id`
- **Уровень доступа:** Локальный токен тенанта (студент или инструктор).
- **Назначение:** Получение деталей завершённой или активной попытки, включая все данные ответов и объяснения к ним.
- **Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "attempt": {
      "id": "uuid-v4",
      "status": "completed",
      "score": 1.0,
      "passed": true
    },
    "answers": [
      {
        "question_id": "uuid-v4-1",
        "answer_text": "Rust",
        "is_correct": true,
        "points_earned": 10,
        "explanation": null
      }
    ]
  },
  "error": null
}
```

#### `GET /api/v1/courses/:course_id/users/:user_id/attempts`
- **Уровень доступа:** Локальный токен тенанта (инструктор, администратор или сам студент).
- **Назначение:** Получение истории всех попыток пользователя по конкретному курсу, отсортированных по дате начала (новые первыми).
- **Response (200 OK):** Массив объектов `Attempt`.

---

## 5. Feature Flags

### 5.1. Глобальное управление (супер-администратор)

#### `GET /api/v1/internal/features`
#### `PATCH /api/v1/internal/features/{name}`
#### `POST /api/v1/internal/features/{name}/tenant/{tenant_id}`
#### `DELETE /api/v1/internal/features/{name}/tenant/{tenant_id}`
- **Уровень доступа:** Глобальный токен (`infrastructure:provisioning`).
- **Назначение:** Управление глобальными флагами и тенантными override (см. `FEATURE_FLAGS.md` §5.1).

### 5.2. Тенантное управление (делегированные флаги)

#### `GET /api/v1/features`
#### `PATCH /api/v1/features/{name}`
- **Уровень доступа:** Локальный токен тенанта со scope `features:write`.
- **Ограничение:** Только для флагов с `is_delegatable = true` (см. `FEATURE_FLAGS.md` §5.2).

---

## 6. Лицензирование (On-Premise / Air-gapped)

### 6.1. Просмотр статуса лицензии

#### `GET /api/v1/internal/license`
- **Уровень доступа:** Глобальный токен (`infrastructure:monitor`) или локальный токен тенанта со scope `license:read`.
- **Назначение:** Просмотр текущего состояния лицензии (см. `LICENSING.md` §7.1).

### 6.2. Установка новой лицензии (только On-Premise)

#### `POST /api/v1/internal/license`
- **Уровень доступа:** Глобальный токен (`infrastructure:provisioning`).
- **Назначение:** Установка новой лицензии после продления или ротации (см. `LICENSING.md` §7.2).
- **Request:**
```json
{
  "license_key": "<base64url(header)>.<base64url(payload)>.<base64url(signature)>"
}
```

---

## 7. Событийные Вебхуки (Webhooks Engine)

### 7.1. Управление подписками

#### `GET /api/v1/webhooks`
#### `POST /api/v1/webhooks`
#### `DELETE /api/v1/webhooks/{id}`
- **Уровень доступа:** Локальный токен тенанта.
- **Назначение:** Подписка на события платформы (создание пользователя, завершение курса, выдача сертификата).

### 7.2. Формат вебхука

**Request (от платформы к внешнему сервису):**
```json
{
  "event": "user.created",
  "tenant_id": "uuid-v4",
  "timestamp": "2026-10-02T14:30:00Z",
  "data": {
    "user_id": "uuid-v4",
    "email": "user@example.com"
  },
  "signature": "sha256=..."
}
```
*(Примечание: `user_id` в вебхуках ссылается на запись в таблице `users`, то есть на конкретную роль личности в данном тенанте).*

**Гарантии доставки:**
- At-least-once с retry (exponential backoff: 1s, 5s, 30s, 5min, 30min).
- После 5 неудачных попыток — перемещение в `dead_letter` queue.
- Подпись HMAC-SHA256 для верификации источника.

---

## 8. Ограничения и лимиты

- Максимальное количество API-ключей на тенант: **100** (см. `NFR.md` §4).
- Максимальное количество вебхук-подписок на тенант: **50**.
- Размер одного запроса: **≤ 10 МБ** (для bulk-операций).
- Timeout для всех запросов: **30 секунд** (кроме streaming-экспорта LRS — 5 минут).

---

## 9. Связь с другими спецификациями

- `NFR.md` — лимиты, latency budgets, пропускная способность.
- `DB_SCHEMA.md` — структура таблиц (Identity-First), RLS-политики, схемы Assessments Engine.
- `OFFLINE_SYNC.md` — клиентская логика синхронизации.
- `FEATURE_FLAGS.md` — управление функциональными флагами.
- `LICENSING.md` — офлайн-лицензирование для коробочных поставок.
- `STANDARDS.md` — соответствие SCORM, xAPI, LTI, SCIM.
