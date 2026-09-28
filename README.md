# Модульная LMS-платформа (Full-Stack Rust Core)

Высокопроизводительная образовательная платформа корпоративного уровня (Enterprise-grade), спроектирована на базе изоморфного стека **Full-Stack Rust** с жесткой изоляцией данных организаций (**Strict Multi-tenancy**), двухуровневым рантаймом плагинов (**WebAssembly / iframe**) и архитектурой **Offline-First PWA**.

---

## 🛠️ Технологический стек проекта

*   **Backend Рантайм:** Rust (асинхронный веб-фреймворк `Axum`).
*   **Frontend & RPC Слой:** Rust `Leptos 0.7+` (SSR-рендеринг + клиентская WASM-гидратация, RPC-взаимодействие через `#[server]`-функции).
*   **Стилизация интерфейса:** `Tailwind CSS` (динамические дизайн-системы тенантов).
*   **Основная база данных (Core):** `PostgreSQL 15+` с аппаратной изоляцией строк через политики `Row-Level Security (RLS)`.
*   **Аналитическое хранилище (LRS):** Инвариантный слой с поддержкой `TimescaleDB` (гипертаблицы) или `ClickHouse` (OLAP-кластер) для обработки xAPI логов.
*   **Локальное хранилище PWA:** `IndexedDB` (через экосистему `web-sys / gloo`) для удержания транзакционных офлайн-очередей.

---

## 📦 Архитектура Cargo Workspace (Структура крейтов)

Проект разработан в виде монорепозитория, разделенного на шесть специализированных микро-крейтов:

*   `crates/shared` — плоские DTO (Data Transfer Object), сущности (Users, Courses, Batches), контракты обмена и xAPI JSON-LD структуры.
*   `crates/ui` — общая UI-библиотека атомарных компонентов и блоков верстки хоста (Tailwind CSS, доступность, Fluent-локализация).
*   `crates/api` — ядро бизнес-логики, слой взаимодействия с базами данных (PostgreSQL + RLS менеджер) и LRS-аналитика. **Без зависимостей от Leptos.**
*   `crates/admin` — изолированные изоморфные компоненты, роуты и `#[server]`-функции панели управления и администрирования тенантов.
*   `crates/client` — изоморфные компоненты, роуты и `#[server]`-функции витрины обучения, PWA-клиента и рантайма плагинов.
*   `crates/server` — точка входа бэкенда хоста на Axum, инициализация пулов, интеграция SSO вебхуков и запуск планировщиков Tokio.

---

## 🧭 Навигация по спецификациям и ТЗ

Все архитектурные регламенты и файлы Технического Задания расположены в каталоге **`specs/`**:

*   [`specs/SPECIFICATION.md`](specs/SPECIFICATION.md) — Бизнес-концепция, иерархия образовательных программ, когорт (`batches`) и требования.
*   [`specs/STRUCTURE.md`](specs/STRUCTURE.md) — Физическая карта папок проекта и правила направленности зависимостей (Dependency Rules).
*   [`specs/DB_SCHEMA.md`](specs/DB_SCHEMA.md) — Спецификация реляционных таблиц, триггеров, политик RLS и структуры LRS аналитики.
*   [`specs/OPEN_API.md`](specs/OPEN_API.md) — Контракты эндпоинтов REST/GraphQL, выдача Opaque-токенов и воркер вебхуков.
*   [`specs/OFFLINE_SYNC.md`](specs/OFFLINE_SYNC.md) — Границы работы PWA без сети, схемы IndexedDB и менеджер пакетной синхронизации.
*   [`specs/PLUGIN.md`](specs/PLUGIN.md) — Двухуровневый гибридный рантайм плагинов (песочница `iframe` / динамический `WebAssembly`) и конечный автомат (FSM).
*   [`specs/PLUGIN_DEVELOPMENT_TEMPLATE.md`](specs/PLUGIN_DEVELOPMENT_TEMPLATE.md) — Шаблон ТЗ и UI/UX регламент для команд-разработчиков внешних плагинов.
*   [`specs/DEPLOY.md`](specs/DEPLOY.md) — Инфраструктурный манифест Docker Compose, конфигурации Ingress Nginx, заголовки CSP и регламент Air-gapped On-Premise.
*   [`specs/STATUS.md`](specs/STATUS.md) — Матрица текущей готовности фич и слоев системы.
*   [`specs/RBAC.md`](specs/RBAC.md) — Матрица ролей и доступов внутри тенантов.
*   [`specs/DIAGNOSTICS.md`](specs/DIAGNOSTICS.md) — Регламент сквозного структурированного логирования (`tracing`).
*   [`specs/GOTCHAS.md`](specs/GOTCHAS.md) — Журнал зафиксированных технических ловушек сборки и рантайма.

---

## 🚀 Быстрый запуск в режиме разработки (Локальный запуск)

```bash
# 1. Поднятие СУБД из корня проекта
docker compose -f specs/DEPLOY.md up -d core-postgres-db dam-object-storage

# 2. Полная проверка воркспейса
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```
