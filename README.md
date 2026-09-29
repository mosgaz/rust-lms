# Модульная мультиарендная LMS-платформа (Full-Stack Rust Core)

Высокопроизводительная образовательная платформа корпоративного уровня (Enterprise-grade), спроектирована на базе изоморфного стека **Full-Stack Rust** с жесткой изоляцией данных организаций (**Strict Multi-tenancy**), двухуровневым рантаймом плагинов (**WebAssembly / iframe**) и архитектурой **Offline-First PWA**.

---

## 🛠️ Технологический стек проекта

*   **Backend Хост (Сетевой рантайм):** Rust (асинхронный веб-фреймворк `Axum`).
*   **Application UI & RPC Слой:** Rust `Leptos 0.7+` (SSR-рендеринг + клиентская WASM-гидратация, RPC-взаимодействие через `#[server]`-функции).
*   **Стилизация интерфейса:** `Tailwind CSS` (динамические дизайн-системы тенантов).
*   **Основная база данных (Core):** `PostgreSQL 15+` с аппаратной изоляцией строк через политики `Row-Level Security (RLS)`.
*   **Аналитическое хранилище (LRS):** Инвариантный слой с поддержкой `TimescaleDB` (гипертаблицы) или `ClickHouse` (OLAP-кластер) для обработки xAPI логов.
*   **Локальное хранилище PWA:** `IndexedDB` (через экосистему `web-sys / gloo`) для удержания транзакционных офлайн-очередей.

---

## 📦 Архитектура Cargo Workspace (Структура крейтов)

Проект разработан в виде монорепозитория, разделенного на пять специализированных микро-крейтов:

*   `crates/shared` — плоские DTO (Data Transfer Object), сущности (Users, Courses, Batches), контракты обмена и xAPI JSON-LD структуры. **Абсолютный ноль внешних зависимостей.**
*   `crates/ui` — чистая библиотека переиспользуемых атомарных компонентов и блоков верстки дизайн-системы (Tailwind CSS, базовая доступность, Fluent-локализация).
*   `crates/api` — серверное ядро бизнес-логики, слой взаимодействия с базами данных (PostgreSQL + RLS менеджер) и LRS-аналитика. **Без зависимостей от макросов Leptos.**
*   `crates/client` — изоморфное full-stack веб-приложение на Leptos. Отвечает за маршрутизацию, авторизацию и UI. Включает публичный сайт (`website`), студенческий кабинет (`student`) и лениво загружаемую панель управления (`cpanel`).
*   `crates/server` — точка входа бэкенда хоста на Axum, инициализация СУБД пулов, раздача WASM-статики и запуск планировщиков Tokio.

---

## 🧭 Навигация по спецификациям и ТЗ

Все архитектурные регламенты и файлы Технического Задания расположены в каталоге **`specs/`**:

*   [`specs/SPECIFICATION.md`](specs/SPECIFICATION.md) — Бизнес-концепция, иерархия образовательных программ, когорт (`batches`) и требования.
*   [`specs/STRUCTURE.md`](specs/STRUCTURE.md) — Физическая карта папок проекта, правила направленности зависимостей (Dependency Rules) и Cargo workspace.
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
*   [`CONTRIBUTING.md`](CONTRIBUTING.md) — Стандарты коммитов и ветвления.

---

## 🚀 Быстрый запуск в режиме разработки (Локальный запуск)

```bash
# 1. Поднятие СУБД и DAM из корня проекта
docker compose -f specs/DEPLOY.md up -d core-postgres-db dam-object-storage

# 2. Полная проверка воркспейса перед коммитом
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```
