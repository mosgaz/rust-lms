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
- **Назначение:** Публикация новой версии курса (инкремент `version` в `courses`, см. `DB_SCHEMA.md` §1.2).
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
- `DB_SCHEMA.md` — структура таблиц (Identity-First), RLS-политики.
- `OFFLINE_SYNC.md` — клиентская логика синхронизации.
- `FEATURE_FLAGS.md` — управление функциональными флагами.
- `LICENSING.md` — офлайн-лицензирование для коробочных поставок.
- `STANDARDS.md` — соответствие SCORM, xAPI, LTI, SCIM.
