
# Стандарты кода для AI-ассистентов

Этот документ — **закон**. При генерации кода следуйте ему буквально.
Если задача противоречит этому документу — спросите, а не импровизируйте.

Источник истины по политике `unsafe`, зависимостей и коммитов — [`AGENTS.md`](AGENTS.md). При расхождении приоритет у `AGENTS.md`.

## 1. Rust Core: Базовые правила и Ошибки

### 1.1. Никакого `unsafe` без явного обоснования

В рамках стандартной образовательной бизнес-логики LMS `unsafe`-код практически не нужен. Политика соответствует [`AGENTS.md`](AGENTS.md) §3: `unsafe` запрещён, кроме низкоуровневых WASM-мостов, и в этом случае обязателен комментарий `// SAFETY:` и ревью.

```rust
// ❌ БЕЗ ОБОСНОВАНИЯ — CI отклонит
let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

// ✅ С ОБОСНОВАНИЕМ
// SAFETY: ptr получен из контролируемой WASM-памяти плагина,
// len проверен на границы выделенного буфера, время жизни ограничено scope'ом транзакции.
let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
```

**Правило:** Каждый `unsafe`-блок обязан иметь комментарий `// SAFETY:` с детальным техническим объяснением, **почему** это безопасно. Без комментария — CI аппаратно отклонит коммит.

### 1.2. Предпочтение `&[T]` над `Vec<T>` в аргументах функций

```rust
// ❌ ПЛОХО: заставляет аллоцировать память на куче
fn process_statements(data: Vec<XApiStatement>) { ... }

// ✅ ХОРОШО: принимает и слайс, и Vec, минимизирует копирование
fn process_statements(data: &[XApiStatement]) { ... }
```

### 1.3. Обработка ошибок — `thiserror` + оператор `?`

Внутренние библиотеки, SDK и бизнес-логика возвращают строго типизированные перечисления. Передача ошибок в виде `String` запрещена.

```rust
// ❌ ПЛОХО
fn parse_import_file() -> Result<UserPayload, String> { ... }

// ✅ ХОРОШО
#[derive(Debug, thiserror::Error)]
pub enum ImportMappingError {
    #[error("failed to map field '{field}': expected format {expected}, got '{got}'")]
    InvalidFieldFormat { field: String, expected: String, got: String },
    #[error("unexpected EOF during streaming parse at row {0}")]
    UnexpectedRowEof(u64),
    #[error("unsupported file schema signature: {0}")]
    UnsupportedSchema(String),
}
```

**Граница хоста.** В крейте `server` (точка входа Axum) допускается `anyhow::Result` для склейки ошибок на верхнем уровне. См. [`AGENTS.md`](AGENTS.md) §7.

### 1.4. Никаких `unwrap()` / `expect()` в runtime-коде

```rust
// ❌ ПЛОХО: паника в WASM при отсутствии сети намертво крашит Service Worker и вкладку PWA
let config = local_storage.get_item("sync_config").unwrap();

// ✅ ХОРОШО: безопасная обработка отсутствия данных или ошибки
let config = local_storage.get_item("sync_config").map_err(|e| {
    tracing::error!("Failed to read sync config from storage: {e:?}");
    LmsError::StorageAccessFailed
})?;
```

**Исключение:** `unwrap()` и `expect()` легитимны в тестах (`#[test]` / `#[cfg(test)]`), примерах (`examples/`), бенчмарках (`benches/`) и на этапе fail-fast инициализации в `main.rs` крейта `server`.

### 1.5. Логирование — исключительно через макросы `tracing`

Использование макросов стандартного вывода `println!` или `eprintln!` в рантайме платформы запрещено.

```rust
// ❌ ПЛОХО
println!("Imported {} users for tenant {}", count, tenant_id);

// ✅ ХОРОШО
tracing::info!(users_count = count, %tenant_id, "Custom ETL user sync completed");
tracing::debug!(?mapping_profile, "Parsing configuration layout");
tracing::error!(%err, "LRS transaction processing failed");
```

---

## 2. PostgreSQL & RLS (Row-Level Security): Мультитенантность

### 2.1. Обязательная установка локального контекста тенанта

Перед выполнением любой бизнес-транзакции в СУБД бэкенд-сервер на Rust (Axum) обязан явно передать идентификатор `tenant_id` в открытую сессию пула соединений.

```rust
// ❌ ПЛОХО: Запрос без установки контекста вернёт пустой результат из-за глобальных политик RLS
let users = sqlx::query!("SELECT * FROM users WHERE email = $1", email)
    .fetch_all(&db_pool)
    .await?;

// ✅ ХОРОШО: Установка параметра сессии внутри единой ACID транзакции
let mut tx = db_pool.begin().await?;
sqlx::query!(
    "SELECT set_config('app.current_tenant_id', $1, true)",
    current_tenant_id.to_string()
)
.execute(&mut *tx)
.await?;

let users = sqlx::query!("SELECT * FROM users WHERE email = $1", email)
    .fetch_all(&mut *tx)
    .await?;
tx.commit().await?;
```

**Примечание.** `SET LOCAL app.current_tenant_id = $1` не поддерживает параметризацию через `$1` — используйте `set_config(..., true)` либо интерполяцию с предварительной валидацией `tenant_id` как UUID.

### 2.2. Запрет на обход RLS

Запрещено использовать системные учётные записи СУБД (суперпользователей), игнорирующие политики Row-Level Security, для выполнения регулярных операций тенанта. Пул соединений приложения должен работать под учётной записью с ограниченными правами.

---

## 3. Изоморфный UI: Leptos 0.7+ & WebAssembly

### 3.1. Управление реактивными сигналами (Signals)

При работе с макросами Leptos необходимо избегать утечек памяти и неконтролируемых циклических зависимостей эффектов.

```rust
// ❌ ПЛОХО: Клонирование тяжелых структур внутрь замыкания без явного отслеживания
let (data, set_data) = create_signal(heavy_struct);
create_effect(move |_| {
    let current = data.get();
    process_ui_update(current);
});

// ✅ ХОРОШО: Использование методов .with() для чтения данных по ссылке без клонирования
create_effect(move |_| {
    data.with(|current| {
        process_ui_update(current);
    });
});
```

### 3.2. Асинхронные ресурсы (Leptos Resources)

- **Чтение данных из Open API** — через Leptos `Resource`.
- **Мутации** — через `create_action`.

Оба механизма обеспечивают корректную работу SSR (Server-Side Rendering) и Hydration на клиенте.

### 3.3. Zero-Copy передача данных в WASM-модули плагинов

При взаимодействии Ядра и верифицированных плагинов **только в Контуре Б (динамический WASM)** запрещено использовать тяжелую сериализацию в JSON через текстовые мосты на стороне клиента.

```rust
// ❌ ПЛОХО: Накладные расходы на сериализацию 10 МБ состояния ломают плавность интерфейса
let state_str = serde_json::to_string(&heavy_state)?;
plugin_iframe.post_message(&state_str);

// ✅ ХОРОШО: Передача данных через биндинги и общую WASM-память
use crate::plugin_sdk::WasmMemoryBridge;
let memory_ptr = WasmMemoryBridge::allocate_shared_buffer(heavy_bytes.len());
WasmMemoryBridge::copy_to_shared_memory(memory_ptr, &heavy_bytes);
plugin_module.init_with_shared_buffer(memory_ptr, heavy_bytes.len());
```

**Примечание.** Для Контура А (`iframe`) shared memory невозможна в принципе — там используется `postMessage`. Zero-copy применима только к Контуру Б и требует явной передачи `WebAssembly.Memory` при инстанцировании модуля.

---

## 4. Архитектура Offline-First PWA & IndexedDB

### 4.1. Автономная валидация и запись Statements

При генерации учебного следа в офлайн-режиме Service Worker обязан выполнять локальную проверку структуры xAPI на соответствие схемам и записывать события транзакционно.

```rust
// ✅ Запись в локальный буфер IndexedDB при отсутствии сети
pub async fn queue_offline_statement(statement: XApiStatement) -> Result<(), StorageError> {
    let db = open_indexed_db().await?;
    let tx = db.transaction(&["offline_xapi_statements"], TransactionMode::ReadWrite)?;
    let store = tx.object_store("offline_xapi_statements")?;

    // В поле timestamp записывается точное системное время совершения действия на устройстве
    store.put(&serde_wasm_bindgen::to_value(&statement)?).await?;
    // Транзакция IndexedDB завершается автоматически при выходе из scope.
    Ok(())
}
```

### 4.2. Алгоритм пакетной отправки (Sync Batching)

При вызове процедур синхронизации отправка данных должна осуществляться строго чанками. Запрещено передавать всю локальную базу IndexedDB одним несегментированным HTTP-запросом.

---

## 5. Конвейеры ETL (Кастомный импорт пользователей)

### 5.1. Строго потоковое чтение (Streaming Input)

Запрещено загружать входящие файлы импорта (CSV, XML) целиком в оперативную память сервера в виде единого строкового массива `String`.

```rust
// ❌ ПЛОХО: Файл размером 100 МБ вызовет OOM-падение процесса
let content = std::fs::read_to_string(uploaded_file_path)?;

// ✅ ХОРОШО: Чтение чанками (буферизированный стриминг) через tokio::io::BufReader
use tokio::io::AsyncBufReadExt;
let file = tokio::fs::File::open(uploaded_file_path).await?;
let mut reader = tokio::io::BufReader::new(file);
let mut line = String::new();
while reader.read_line(&mut line).await? > 0 {
    process_etl_row(&line)?;
    line.clear();
}
```

**Примечание.** Для XLSX построчное чтение не применимо (формат — ZIP с XML внутри). XLSX обрабатывается отдельным потоковым парсером (например, `calamine` в streaming-режиме); требование «не грузить целиком в память» сохраняется.

### 5.2. Предварительная аллокация коллекций

В циклах парсинга и обработки чанков данных всегда используйте метод `with_capacity` для минимизации накладных расходов на реаллокацию памяти при росте массивов.

```rust
// ✅ Инициализация вектора под фиксированный размер чанка импорта
let mut user_batch = Vec::with_capacity(500);
```

---

## 6. Тестирование и Валидация

### 6.1. Snapshot-тесты для иерархии курсов и маппинга ETL

`insta` разрешён как dev-зависимость для snapshot-тестов.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_etl_mapping_pipeline() {
        let raw_csv = b"ID;ФИО;Департамент\n123;Иванов Иван;ИТ";
        let profile = load_test_mapping_profile();
        let parsed_users = crate::etl::parse_chunk(raw_csv, &profile).unwrap();

        // Использование insta-снапшотов для контроля структуры выходного JSON
        insta::assert_json_snapshot!(parsed_users);
    }
}
```

### 6.2. Тестирование инвариантности LRS (Иммутабельный подход)

Каждый тест, проверяющий синхронизацию логов из офлайна, обязан верифицировать, что конфликтующие во времени statements устройств сохраняются в базу данных LRS в полном объеме, а дедупликация или перезапись данных отсутствует.

---

## 7. Жесткие запреты (Hard Rules)

**КАТЕГОРИЧЕСКИ ЗАПРЕЩЕНО:**

1. Применение макросов `unwrap()` и `expect()` в runtime-коде бизнес-логики (разрешено только в тестах, примерах, бенчмарках и при fail-fast инициализации `server`).
2. Прямой вывод данных через `println!` или `eprintln!` — используйте структурированные макросы экосистемы `tracing`.
3. Модификация или создание сквозных SQL-запросов, позволяющих считывать данные без явной проверки/установки контекста `tenant_id` (за исключением системных эндпоинтов глобального provisioning-а).
4. Аллокации памяти внутри render loop сложных интерактивных WASM-модулей (все буферы состояний должны быть pre-allocated).
5. Использование типов данных с плавающей точкой `f64` в структурах, предназначенных для передачи в смежные сетевые или бинарные API, спецификация которых не предусматривает `f64` нативно (например, xAPI JSON-LD).
6. Организация рекурсивных связей в структурах данных Образовательных программ и Курсов хоста (разрешена только плоская arena-based иерархия идентификаторов).
7. Использование синхронных HTTP-клиентов или блокирующих файловых операций в основном потоке выполнения асинхронного рантайма Tokio.

---

## 8. Именования и Документация

* Все публичные функции, структуры, трейты и модули в обязательном порядке снабжаются doc-комментариями `///`.
* Для крейтов, публикующих API, в `lib.rs` включается `#![deny(missing_docs)]`. Для бинарного крейта `server` — по усмотрению владельца.
* Названия типов данных и структур — существительные в CamelCase (`XApiStatement`, `MappingProfile`, `BatchEnrollment`).
* Названия функций и методов — глаголы в snake_case (`sync_offline_queue`, `parse_import_stream`).
* Названия булевых переменных и флагов — с префиксами `is_`, `has_`, `can_` (`is_authenticated`, `has_offline_cached`).
* Глобальные константы — строго в `UPPER_SNAKE_CASE` (`MAX_BATCH_SIZE_CHUNK`, `DEFAULT_TOKEN_TTL_SECONDS`).
