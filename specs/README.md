# Спецификации проекта (specs/)

Единая точка входа в документацию. Корневой [`README.md`](../README.md) содержит только обзор проекта, технологический стек и быстрый старт — вся навигация по документации живёт здесь, в `specs/README.md`.

## 🧭 Порядок чтения

### Для человека

1. [`SPECIFICATION.md`](SPECIFICATION.md) — бизнес-концепция, иерархия программ, когорт, роли.
2. [`ARCHITECTURE.md`](ARCHITECTURE.md) — сводный ADD: RLS, Open API, LRS, плагины, ETL.
3. [`NFR.md`](NFR.md) — количественные NFR и SLA: RTO/RPO, concurrency, latency budgets, лимиты, sizing.
4. [`STRUCTURE.md`](STRUCTURE.md) — карта папок, состав Cargo workspace, Dependency Rules.
5. Далее — профильные спецификации по зоне ответственности (см. таблицы ниже).

### Для AI-агента

1. [`AGENTS.md`](AGENTS.md) — первичная точка входа: обязательные шаги перед задачей, запреты, чек-лист.
2. [`SPECIFICATION.md`](SPECIFICATION.md) — источник истины по бизнес-требованиям.
3. [`STRUCTURE.md`](STRUCTURE.md) — структура каталогов и Dependency Rules.
4. [`CODING_STANDARDS.md`](CODING_STANDARDS.md) — правила full-stack Rust, RLS, запреты.
5. Далее — профильные спецификации по зоне ответственности.

---

## 📚 Бизнес и архитектура

| Документ | Назначение |
|:---|:---|
| [`SPECIFICATION.md`](SPECIFICATION.md) | Upper-level требования, концепция мультитенантности, иерархия контента, версионирование, сертификации, retention, Feature Flags. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Сводный ADD: RLS, Open API, LRS, плагины, ETL. Профильные источники истины — ниже. |
| [`NFR.md`](NFR.md) | Нефункциональные требования: SLA, RTO/RPO, concurrency, latency budgets, лимиты, деградация, Sizing Guide. |
| [`STRUCTURE.md`](STRUCTURE.md) | Физическая карта папок, зоны ответственности крейтов, правила изоляции и Dependency Rules. |
| [`STATUS.md`](STATUS.md) | Матрица готовности слоёв и фич (три легенды: документация, реализация, планы). |
| [`ROADMAP.md`](ROADMAP.md) | Плановые направления: аналитика, сертификация, биллинг, поиск, мобильное, аудит, Conformance Testing, a11y, Data Portability, Feature Flags ext. |
| [`FEATURE_FLAGS.md`](FEATURE_FLAGS.md) | Управление функциональными флагами (глобальные + tenant overrides, PostgreSQL + LISTEN/NOTIFY). |
| [`decisions/`](decisions/) | Реестр архитектурных решений (ADR). Точка входа — [`decisions/README.md`](decisions/README.md). |

## 🗄️ Данные и API

| Документ | Назначение |
|:---|:---|
| [`DB_SCHEMA.md`](DB_SCHEMA.md) | Реляционный слой PostgreSQL, RLS, версионирование контента, i18n-метаданные, retention, LRS, feature flags, license, revoked JWT kids. |
| [`MIGRATIONS.md`](MIGRATIONS.md) | Регламент миграций на базе `sqlx` (альтернатива — `SeaORM`) + Runbook для администратора On-Premise. |
| [`OPEN_API.md`](OPEN_API.md) | Контракты REST/GraphQL: provisioning, ETL, LRS, offline-sync, SCIM 2.0, signed-url, вебхуки, версионирование. |

## 🔌 Плагины и рантайм

| Документ | Назначение |
|:---|:---|
| [`PLUGIN.md`](PLUGIN.md) | Двухуровневый рантайм (iframe / WASM), контракты SDK, FSM, подпись/верификация/kill switch, SBOM. |
| [`PLUGIN_DEVELOPMENT_TEMPLATE.md`](PLUGIN_DEVELOPMENT_TEMPLATE.md) | Шаблон ТЗ и UI/UX регламент для внешних команд (включая a11y). |

## 🌐 Клиент, PWA, коммуникации

| Документ | Назначение |
|:---|:---|
| [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) | Границы PWA-офлайна, IndexedDB-очереди, разрешение конфликтов, iOS-ограничения, Signed URLs / DRM, офлайн-шелл формы входа. |
| [`COMMUNICATIONS.md`](COMMUNICATIONS.md) | Чаты, комментарии, уведомления. |
| [`CONFERENCING.md`](CONFERENCING.md) | Встроенная ВКС: WebRTC P2P/SFU, fallback, ограничения MVP, локальные TURN/STUN. |

## 📜 Стандарты, роли, качество

| Документ | Назначение |
|:---|:---|
| [`STANDARDS.md`](STANDARDS.md) | SCORM (Режим 1 / Режим 2), xAPI, LTI, SCIM 2.0, WCAG 2.2 AA, i18n/l10n, GDPR/CCPA, retention, Conformance Testing, Data Portability. |
| [`RBAC.md`](RBAC.md) | Матрица ролей (Администратор, Инструктор, Ментор, Обучающийся, Наблюдатель) внутри тенантов. |
| [`DIAGNOSTICS.md`](DIAGNOSTICS.md) | Сквозное структурированное логирование (`tracing`), распределённый трейсинг (OpenTelemetry). |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | Правила full-stack Rust, запреты, управление памятью. |
| [`GOTCHAS.md`](GOTCHAS.md) | Журнал зафиксированных технических ловушек сборки и рантайма. |

## 🚀 Инфраструктура и процесс

### Внутренние документы `specs/`

| Документ | Назначение |
|:---|:---|
| [`DEPLOY.md`](DEPLOY.md) | Docker Compose, Ingress Nginx, CSP, COOP/COEP, Air-gapped On-Premise, Vault, OTel, обновления, DR, сертификаты, управление криптоключами, runbook компрометации JWT, Chaos Engineering. |
| [`LICENSING.md`](LICENSING.md) | Подсистема офлайн-лицензирования для коробочных поставок (формат ключа, привязка, enforcement, grace period, связь с Feature Flags). |
| [`AGENTS.md`](AGENTS.md) | Инструкции для AI-агентов (Claude Code, Cursor, Cline, Codex, Copilot). |
| [`docker-compose.yml`](docker-compose.yml) | Манифест локального / On-Premise развёртывания (команда запуска — в корневом [`README.md`](../README.md)). |

### Внешние документы (корень репозитория)

| Документ | Назначение |
|:---|:---|
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | Стандарты коммитов и ветвления (Conventional Commits). |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Журнал изменений (заполняется по Conventional Commits). |