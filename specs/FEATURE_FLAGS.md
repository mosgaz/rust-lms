# Управление функциональными флагами (Feature Flags)

**Файл спецификации:** `FEATURE_FLAGS.md`

> Смежные разделы: разграничение с лицензированием — в [`LICENSING.md`](LICENSING.md); структура таблиц — в [`DB_SCHEMA.md`](DB_SCHEMA.md) §1.2; API — в [`OPEN_API.md`](OPEN_API.md) §3; архитектурное решение — в ADR [`2026.09.29-0009.md`](decisions/2026.09.29-0009.md); RLS — в ADR [`2026.09.28-0001.md`](decisions/2026.09.28-0001.md); Revocation List плагинов — в [`PLUGIN.md`](PLUGIN.md) §7.3.

## 1. Назначение и концепция

Feature Flags (функциональные флаги) — это механизм управления **операционной доступностью** функций платформы **без релиза нового кода**. Они используются для:

* Безопасного поэтапного rollout (например, включение SCIM для одного тенанта).
* Закрытых бета-тестов (например, ВКС для выбранных организаций).
* Временного отключения функции при инцидентах без отката релиза.
* A/B-тестирования (например, разные варианты UI для разных тенантов).

### 1.1. Разграничение с `license.features`

Платформа использует **два независимых механизма** для контроля функций:

| Механизм | Что определяет | Источник | Управление |
|:---|:---|:---|:---|
| **`license.features`** | **Коммерческое право** — что тенант купил. | Лицензионный ключ (см. [`LICENSING.md`](LICENSING.md)). | Вендор при выпуске лицензии. |
| **`feature_flags`** | **Операционное состояние** — что включено прямо сейчас. | PostgreSQL. | Супер-админ (глобально) / тенант-админ (для делегированных флагов). |

**Каскадная логика:**

```
Право (license)  →  Состояние (feature_flags)  →  Бизнес-логика
```

* Если лицензия **не даёт право** — функция недоступна **всегда** (`403 feature_not_licensed`), независимо от флага.
* Если лицензия **даёт право**, но флаг **выключен** — функция недоступна **временно** (`403 feature_not_enabled`).
* Если лицензия **даёт право** и флаг **включён** — функция работает.

**Пример.** Тенант купил ВКС (`license.features.vks = true`). Но ВКС в beta-тесте, и feature flag `vks` глобально `false`. Для одного тенанта (участника beta) установлен override `vks = true`. Результат:
* для тенанта-участника — ВКС работает;
* для остальных тенантов с той же лицензией — `403 feature_not_enabled`.

### 1.2. Два уровня флагов

* **Глобальные флаги** (`feature_flags`) — общие для всей платформы.
  * `enabled` — значение по умолчанию для всех тенантов.
  * `is_delegatable` — может ли тенант-админ переключать флаг для своего тенанта.
* **Тенантные переопределения** (`tenant_feature_flags`) — специфичны для конкретного тенанта.
  * Если override существует — применяется он.
  * Если override отсутствует — применяется `enabled` из глобальной таблицы (правило наследования, Вариант A из ADR-0009).

## 2. Структура данных

> **Источник истины по схеме:** [`DB_SCHEMA.md`](DB_SCHEMA.md) §1.2. Ниже — сводка для контекста.

### 2.1. Таблица `feature_flags` (глобальная, вне RLS)

| Поле | Тип | Описание |
|:---|:---|:---|
| `id` | UUID (PK) | Идентификатор флага. |
| `name` | VARCHAR(64) UNIQUE | Строковый идентификатор (например, `vks`, `scim`, `ale`, `conformance_testing`). |
| `description` | TEXT | Человекочитаемое описание (для админ-панели). |
| `enabled` | BOOLEAN NOT NULL DEFAULT FALSE | Значение по умолчанию для всех тенантов. |
| `is_delegatable` | BOOLEAN NOT NULL DEFAULT FALSE | Может ли тенант-админ переключать флаг. |
| `created_at` / `updated_at` | TIMESTAMPTZ | Метки времени. |

### 2.2. Таблица `tenant_feature_flags` (защищена RLS)

| Поле | Тип | Описание |
|:---|:---|:---|
| `id` | UUID (PK) | Идентификатор записи. |
| `tenant_id` | UUID (FK -> tenants.id ON DELETE CASCADE) | Тенант. |
| `flag_name` | VARCHAR(64) (FK -> feature_flags.name) | Имя флага. |
| `enabled` | BOOLEAN NOT NULL | Значение override. |
| `updated_by` | UUID (FK -> users.id, NULLable) | Кто изменил (если через UI/API) или NULL (если через CLI супер-админом). |
| `updated_at` | TIMESTAMPTZ | Метка времени. |

**Ограничения:** `UNIQUE (tenant_id, flag_name)`.

**RLS-политика:** та же, что для остальных тенант-таблиц (см. [`DB_SCHEMA.md`](DB_SCHEMA.md) §1.1).

## 3. Правило наследования

При проверке флага `F` для тенанта `T`:

1. Ищем запись в `tenant_feature_flags` по `(T, F)`.
2. Если запись **есть** — используем её значение `enabled`.
3. Если записи **нет** — используем `enabled` из `feature_flags` по `F`.

Это правило Варианта A из ADR-0009: override → global → default.

**Отдельно:** `is_delegatable` — свойство **глобального** флага. Если `is_delegatable = true`, тенант-админ может создать override. Если `false` — не может, флаг отображается в `cpanel` как «заблокировано администратором платформы».

## 4. Делегирование управления

| Флаг | `is_delegatable` | Кто управляет |
|:---|:---:|:---|
| `vks` (beta) | `false` | Только супер-админ (закрытый beta-тест). |
| `vks` (общий релиз) | `true` | Тенант-админ сам включает/выключает ВКС для своей организации. |
| `scim` | `false` | Только супер-админ (функция интеграции с HRIS). |
| `ale` | `true` | Тенант-админ включает шифрование PII для своей организации. |
| `conformance_testing` | `true` | Тенант-админ сам решает, использовать ли CLI/сервис. |
| `advanced_analytics` | `true` | Тенант-админ включает/выключает расширенную аналитику. |

Значения `is_delegatable` устанавливаются супер-админом при создании флага и могут быть изменены только им.

## 5. API-контракты

### 5.1. Супер-администратор (глобальные операции)

#### `GET /api/v1/internal/features`

* **Уровень доступа:** Глобальный токен (`infrastructure:monitor`).
* **Назначение:** Список всех глобальных флагов с их состоянием.
* **Ответ:**
  ```json
  {
    "flags": [
      { "name": "vks", "description": "...", "enabled": false, "is_delegatable": false },
      { "name": "ale", "description": "...", "enabled": true,  "is_delegatable": true }
    ]
  }
  ```

#### `PATCH /api/v1/internal/features/{name}`

* **Уровень доступа:** Глобальный токен (`infrastructure:provisioning`).
* **Назначение:** Изменение глобального флага.
* **Входящий payload:** `{ "enabled": true, "is_delegatable": false }` (любое поле опционально).
* **Поведение:** обновляет запись в `feature_flags`, шлёт `NOTIFY feature_flags_changed` с payload `{"flag_name": "<name>"}`, пишет запись в `audit_log`.
* **Ответ:** `200 OK` с обновлённым состоянием.

#### `POST /api/v1/internal/features/{name}/tenant/{tenant_id}`

* **Уровень доступа:** Глобальный токен (`infrastructure:provisioning`).
* **Назначение:** Установка override для тенанта.
* **Входящий payload:** `{ "enabled": true }`.
* **Поведение:** UPSERT в `tenant_feature_flags`, `NOTIFY feature_flags_changed` с payload `{"flag_name": "<name>", "tenant_id": "<uuid>"}`, запись в `audit_log`.
* **Ответ:** `200 OK`.

#### `DELETE /api/v1/internal/features/{name}/tenant/{tenant_id}`

* **Назначение:** Удаление override (возврат к глобальному значению).
* **Поведение:** DELETE из `tenant_feature_flags`, `NOTIFY`, запись в `audit_log`.
* **Ответ:** `204 No Content`.

### 5.2. Тенант-администратор (делегированные флаги)

#### `GET /api/v1/features`

* **Уровень доступа:** Локальный токен тенанта.
* **Назначение:** Список флагов, видимых тенанту (включая `is_delegatable`).
* **Ответ:** флаги с полями `name`, `description`, `enabled` (текущее значение для тенанта), `is_delegatable`, `source` (`override` / `global`).

#### `PATCH /api/v1/features/{name}`

* **Уровень доступа:** Локальный токен тенанта со scope `features:write` (см. [`RBAC.md`](RBAC.md)).
* **Ограничение:** только если `is_delegatable = true` для этого флага. Иначе — `403 feature_not_delegatable`.
* **Входящий payload:** `{ "enabled": true }`.
* **Поведение:** UPSERT в `tenant_feature_flags` с `updated_by = current_user_id`, `NOTIFY`, запись в `audit_log`.
* **Ответ:** `200 OK`.

## 6. CLI `rust-lms-cli features`

Для Air-gapped и административных операций.

```bash
# Список флагов
rust-lms-cli features list

# Показать состояние конкретного флага
rust-lms-cli features show vks

# Включить глобальный флаг
rust-lms-cli features enable vks

# Выключить глобальный флаг
rust-lms-cli features disable vks

# Установить override для тенанта
rust-lms-cli features set --tenant <tenant_id> --flag vks --enabled true

# Удалить override (вернуть к глобальному значению)
rust-lms-cli features unset --tenant <tenant_id> --flag vks

# Изменить is_delegatable
rust-lms-cli features set-delegatable --flag vks --value true
```

CLI подключается к БД напрямую (локально, без API), выполняет операции в транзакции, отправляет `NOTIFY` тем же каналом.

## 7. Каскадная проверка в коде (Rust/Axum)

```rust
// В middleware или явной проверке в хэндлере
pub async fn require_feature(
    state: &AppState,
    tenant_id: TenantId,
    feature: &str,
) -> Result<(), ApiError> {
    // 1. Проверка коммерческого права
    if !state.license_cache.has_feature(feature) {
        return Err(ApiError::Forbidden {
            error: "feature_not_licensed",
            feature: feature.to_string(),
        });
    }

    // 2. Проверка операционного состояния
    let enabled = state
        .feature_flag_cache
        .is_enabled_for_tenant(tenant_id, feature)
        .await?;

    if !enabled {
        return Err(ApiError::Forbidden {
            error: "feature_not_enabled",
            feature: feature.to_string(),
        });
    }

    // 3. Техническая деградация (проверяется в самом хэндлере по обстоятельствам)
    // ...
    Ok(())
}
```

**HTTP-ответы:**

| Ситуация | Код | Тело |
|:---|:---:|:---|
| Нет в лицензии | `403 Forbidden` | `{"error": "feature_not_licensed", "feature": "vks"}` |
| Флаг выключен | `403 Forbidden` | `{"error": "feature_not_enabled", "feature": "vks"}` |
| Флаг не делегируемый (для тенант-админа) | `403 Forbidden` | `{"error": "feature_not_delegatable", "feature": "vks"}` |
| Техническая деградация | `503 Service Unavailable` | `{"error": "feature_unavailable", "feature": "vks", "reason": "..."}` |

## 8. In-memory кэш и синхронизация

### 8.1. Структура кэша

```rust
pub struct FeatureFlagCache {
    /// Глобальные флаги: name -> (enabled, is_delegatable)
    global: HashMap<String, GlobalFlag>,

    /// Тенантные переопределения: (tenant_id, name) -> enabled
    /// При > 100 000 тенантов заменяется на moka::sync::Cache (LRU).
    tenant_overrides: HashMap<(TenantId, String), bool>,

    /// Флаг готовности кэша. Пока false — проверка идёт напрямую в БД.
    cache_ready: AtomicBool,

    /// Метка последней успешной синхронизации.
    last_sync_at: RwLock<Instant>,
}

pub struct GlobalFlag {
    pub enabled: bool,
    pub is_delegatable: bool,
}
```

### 8.2. Инициализация

* При старте инстанса `api` — **синхронная загрузка** всех флагов из БД (до порога 100 000 тенантов).
* После загрузки — `cache_ready = true`.
* Пока `cache_ready = false` — проверка флага идёт напрямую в БД (медленнее, но безопасно).

### 8.3. Обновление: LISTEN/NOTIFY

* Создаётся **одно выделенное соединение** `sqlx::PgConnection` (owned, вне пула).
* `LISTEN feature_flags_changed; LISTEN license_changed;`
* Логика переподключения — exponential backoff (1s, 2s, 4s, …, max 60s).
* **Payload NOTIFY:**
  * Для глобального флага: `{"flag_name": "vks"}` — инстанс делает `SELECT * FROM feature_flags WHERE name = 'vks'` и обновляет одну запись в кэше.
  * Для override: `{"flag_name": "vks", "tenant_id": "<uuid>"}` — точечный SELECT и обновление одной пары в `tenant_overrides`.
  * Для `license_changed`: payload пустой (или `{}`), инстанс перечитывает всю лицензию.

### 8.4. Периодическая синхронизация (Polling)

* Фоновая задача `tokio::spawn` раз в **60 секунд** выполняет полное перечитывание состояния флагов из БД и обновляет кэш.
* Это страховка от потери NOTIFY при сетевых разрывах.
* **Окно рассинхронизации между инстансами — до 60 секунд.** Признано допустимым для feature flags.

### 8.5. Sanity check

После любого обновления кэша выполняется проверка:

* Количество глобальных флагов не изменилось внезапно (например, не упало с 50 до 1).
* Значения в допустимых пределах (BOOLEAN).
* При аномалии — fallback на полное перечитывание из БД и запись в `audit_log` с `%reason = 'cache_poisoning_detected'`.

### 8.6. Масштабирование кэша

* До 100 000 тенантов — `HashMap<(TenantId, String), bool>`.
* Свыше — `moka::sync::Cache` с LRU-политикой. При этом `cache_ready` может быть `false` для тенантов, которых нет в LRU, — в этом случае проверка идёт напрямую в БД.

## 9. Аудит изменений

Каждое изменение флага (глобального или override) фиксируется в `audit_log`:

* `%operation` = `feature_flag_updated` | `feature_flag_tenant_override_set` | `feature_flag_tenant_override_unset` | `feature_flag_delegatable_changed`.
* `%tenant_id` — если операция затрагивает конкретный тенант (для override).
* `%user_id` — кто выполнил (супер-админ или тенант-админ).
* `%flag_name` — имя флага.
* `%old_value` / `%new_value` — предыдущее и новое значение.
* `%source` = `api` | `cli`.

Записи в `audit_log` не подчиняются feature flags — они всегда пишутся.

## 10. Связь с другими механизмами

### 10.1. Лицензирование

* `license.features` — право (коммерческое).
* `feature_flags` — состояние (операционное).
* Порядок проверки: лицензия → флаги → бизнес-логика.
* Изменение лицензии (например, продление или расширение пакета) обновляет `license_cache` через канал `license_changed`.

### 10.2. Revocation List плагинов

* Revocation List (см. [`PLUGIN.md`](PLUGIN.md) §7.3) — отдельный механизм, **не** реализуется через feature flags.
* Revocation List подписывается release-ключом и доставляется отдельно (в Air-gapped — тем же каналом, что обновления ядра).
* Feature flag `enable_plugin_<name>` (если появится) может управлять **доступностью** плагина, но **не** его отзывом.

### 10.3. Настройки тенанта

* Feature flags **не заменяют** настройки тенанта (`branding_config`, `sso_config`, `retention_policies`).
* Настройки — это конфигурация, а флаги — это доступность функциональности.
* Например, `allow_offline = false` (свойство курса) — это настройка; `offline_media_caching` (флаг) — это доступность функции в принципе.

## 11. Ограничения и лимиты

* Максимальное количество глобальных флагов — **200** (практически достаточно для любой LMS).
* Максимальное количество override на тенант — **200**.
* Максимальное количество тенантов с override — **100 000** (до порога LRU).
* Payload NOTIFY — не более 200 байт (запас от лимита PostgreSQL в 8000 байт).
* Окно рассинхронизации кэшей между инстансами — **до 60 секунд**.
* Частота polling — **60 секунд** (не настраивается; при необходимости — ADR).

## 12. Связь с ADR

Архитектурное решение по Feature Flags зафиксировано в ADR [`2026.09.29-0009.md`](decisions/2026.09.29-0009.md). ADR фиксирует:

* Вариант B (глобальные + tenant overrides).
* PostgreSQL как source of truth.
* In-memory кэш + LISTEN/NOTIFY.
* Делегирование через `is_delegatable`.
* Каскадную проверку `license → feature_flags → business logic`.
* Нюансы LISTEN/NOTIFY (выделенное соединение, polling, `max_connections`).
* Риски: рассинхронизация до 60 секунд, отравление кэша, пиковая нагрузка.