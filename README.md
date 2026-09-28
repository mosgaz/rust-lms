## README.md (Корневой путеводитель по проекту)

# Модульная LMS-платформа «Nexus» (Full-Stack Rust Core)
Высокопроизводительная образовательная платформа корпоративного уровня (Enterprise-grade), спроектирована на базе изоморфного стека **Full-Stack Rust** с жесткой изоляцией данных организаций (**Strict Multi-tenancy**), двухуровневым рантаймом плагинов (**WebAssembly / iframe**) и архитектурой **Offline-First PWA**.
---## 🛠️ Технологический стек проекта
*   **Backend Хоста:** Rust (асинхронный веб-фреймворк `Axum` или `Actix-web`).
*   **Frontend Хоста:** Rust `Leptos 0.7+` (SSR-рендеринг + клиентская WASM-гидратация).
*   **Стилизация интерфейса:** `Tailwind CSS` (динамические дизайн-системы тенантов).
*   **Основная база данных (Core):** `PostgreSQL 15+` с аппаратной изоляцией строк через политики `Row-Level Security (RLS)`.
*   **Аналитическое хранилище (LRS):** Инвариантный слой с поддержкой `TimescaleDB` (гипертаблицы) или `ClickHouse` (OLAP-кластер) для обработки xAPI логов.
*   **Локальное хранилище PWA:** `IndexedDB` (через экосистему `web-sys / gloo`) для удержания транзакционных офлайн-очередей.
---## 📦 Архитектура Cargo Workspace (Структура крейтов)
Проект разработан в виде монорепозитория, разделенного на независимые микро-крейты (размер каждого `crates/*` ≤ 1500 строк бизнес-логики):

*   `crates/lms-core-backend` — серверное ядро платформы, маршрутизация, менеджмент токенов, SSO и Open API шлюз.
*   `crates/lms-core-frontend` — клиентское прогрессивное веб-приложение (PWA) на Leptos, Service Workers и логика Offline-First.
*   `crates/lms-shared-types` — общие контракты данных, сущности (Users, Courses, Batches), DTO и xAPI JSON-LD структуры.
*   `crates/lms-database-db` — слой взаимодействия с PostgreSQL, миграции структуры и интерцептор сессионного контекста RLS.
*   `crates/lms-lrs-analytics` — хранилище и парсеры учебного следа стандарта xAPI (Experience API / IEEE 2900.1).
*   `crates/lms-plugin-sdk` — SDK и рантайм для изоляции и управления жизненным циклом микроприложений.
*   `crates/lms-etl-mapper` — потоковый конвейер кастомного импорта пользователей (CSV/XLSX ETL) без перегрузки RAM.
*   `crates/lms-task-worker` — асинхронные фоновые воркеры для обработки тяжелых очередей (Tokio-tasks / River).
---## 🧭 Навигация по спецификациям и ТЗ
Все архитектурные регламенты и файлы Технического Задания расположены в каталоге **`specs/`**:

*   [`specs/SPECIFICATION.md`](specs/SPECIFICATION.md) — Бизнес-концепция, иерархия образовательных программ, когорт (`batches`) и требования.
*   [`specsSTRUCTURE.md`](specs/STRUCTURE.md) — Физическая карта папок проекта и правила направленности зависимостей (Dependency Rules).
*   [`specs/DB_SCHEMA.md`](specs/DB_SCHEMA.md) — Спецификация реляционных таблиц, триггеров, политик RLS и структуры LRS аналитики.
*   [`specs/OPEN_API.md`](specs/OPEN_API.md) — Контракты эндпоинтов REST/GraphQL, выдача Opaque-токенов и воркер вебхуков.
*   [`specs/OFFLINE_SYNC.md`](specs/OFFLINE_SYNC.md) — Границы работы PWA без сети, схемы IndexedDB и менеджер пакетной синхронизации.
*   [`specs/PLUGIN.md`](specs/PLUGIN.md) — Двухуровневый гибридный рантайм плагинов (песочница `iframe` / динамический `WebAssembly`) и конечный автомат (FSM).
*   [`specs/PLUGIN_DEVELOPMENT_TEMPLATE.md`](specs/PLUGIN_DEVELOPMENT_TEMPLATE.md) — Шаблон ТЗ и UI/UX регламент для команд-разработчиков внешних плагинов.
*   [`specs/DEPLOY.md`](specs/DEPLOY.md) — Инфраструктурный манифест Docker Compose, конфигурации Ingress Nginx, заголовки CSP и регламент Air-gapped On-Premise.
---## 🚦 Инструкции для Разработчиков и AI-агентов
Если вы работаете в данном репозитории как разработчик или AI-ассистент (Claude Code, Cursor, Cline, Copilot), вы **обязаны** строго следовать управляющим инструкциям:

1.  **Главная точка входа:** Ознакомьтесь с файлом инструкции [`specs/AGENTS.md`](specs/AGENTS.md) — там зафиксированы пред-коммитные чек-листы и критерии выполненной задачи.
2.  **Закон кодинга:** Следуйте правилам full-stack Rust разработки, описанным в [`specs/CODING_STANDARDS.md`](specs/CODING_STANDARDS.md). Любые нарушения стандартов обработки ошибок (`thiserror`), логирования (`tracing`) или RLS будут автоматически заблокированы на этапе CI.
---## 🚀 Быстрый запуск в режиме разработки (Локальный запуск)### 1. Предварительные требованияУбедитесь, что в вашей системе установлены: `Rust stable` (последний релиз), `Docker`, `Docker Compose`, инструмент `cargo-expand` и утилита сборки `wasm-pack`.
### 2. Поднятие инфраструктурного окруженияЗапустите локальные персистентные СУБД (PostgreSQL с RLS + аналитический слой) и объектное хранилище MinIO:```bash
docker compose -f specs/DEPLOY.md up -d nexus-postgres-core nexus-dam-storage
```
### 3. Запуск верификационного конвейера (Тесты и Линтеры)Перед выполнением коммита в обязательном порядке выполните полную проверку воркспейса:```bash
# 1. Проверка компиляции и типов данных
cargo check --workspace --all-targets --all-features

# 2. Строгий статический анализ кода (Линтер)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. Проверка форматирования исходного кода
cargo fmt --all -- --check

# 4. Запуск всех модульных, интеграционных и снапшот-тестов воркспейса
cargo test --workspace
```
### 4. Тестовая компиляция клиентского WASM-бандла PWAПроверьте корректность компиляции Leptos-фронтенда в WebAssembly и убедитесь, что размер артефакта не превышает лимиты:```bash
wasm-pack build --target web crates/lms-core-frontend --release
ls -lh crates/lms-core-frontend/pkg/*.wasm
```
