# Спецификации проекта (specs/)

Единая точка входа в документацию. Корневой [`README.md`](../README.md) — вся навигация живёт здесь.

## 🧭 Порядок чтения

Для нового участника или AI-агента:

1. [`SPECIFICATION.md`](SPECIFICATION.md) — бизнес-концепция, иерархия программ, когорт.
2. [`STRUCTURE.md`](STRUCTURE.md) — карта папок, состав Cargo workspace, Dependency Rules.
3. [`AGENTS.md`](AGENTS.md) — инструкции для AI-агентов.
4. `../CONTRIBUTING.md` — коммиты, ветвление, Conventional Commits.
5. Далее — спецификации по зоне ответственности (см. таблицу ниже).

---

## 📚 Бизнес и архитектура

| Документ | Назначение |
|:---|:---|
| [`SPECIFICATION.md`](SPECIFICATION.md) | Upper-level требования, концепция мультитенантности, иерархия контента и сертификаций. |
| [`STRUCTURE.md`](STRUCTURE.md) | Физическая карта папок, зоны ответственности крейтов, правила изоляции и Dependency Rules. |
| [`STATUS.md`](STATUS.md) | Матрица готовности слоёв и фич (проектирование / реализация / план). |
| [`ROADMAP.md`](ROADMAP.md) | Плановые направления: аналитика, сертификация, биллинг, поиск, мобильное, аудит. |
| [`decisions/`](decisions/) | Реестр архитектурных решений (ADR). Точка входа — [`decisions/README.md`](decisions/README.md). |

## 🗄️ Данные и API

| Документ | Назначение |
|:---|:---|
| [`DB_SCHEMA.md`](DB_SCHEMA.md) | Реляционный слой PostgreSQL, Row-Level Security (RLS), схемы хранения xAPI в LRS. |
| [`MIGRATIONS.md`](MIGRATIONS.md) | Регламент миграций на базе `sqlx` (альтернатива — `SeaORM`). |
| [`OPEN_API.md`](OPEN_API.md) | Контракты REST/GraphQL, provisioning тенантов, Opaque-токены, воркер вебхуков. |

## 🔌 Плагины и рантайм

| Документ | Назначение |
|:---|:---|
| [`PLUGIN.md`](PLUGIN.md) | Двухуровневый рантайм (iframe / WASM), контракты SDK, FSM. |
| [`PLUGIN_DEVELOPMENT_TEMPLATE.md`](PLUGIN_DEVELOPMENT_TEMPLATE.md) | Шаблон ТЗ и UI/UX регламент для внешних команд. |

## 🌐 Клиент, PWA, коммуникации

| Документ | Назначение |
|:---|:---|
| [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) | Границы PWA-офлайна, IndexedDB-очереди, разрешение конфликтов. |
| [`COMMUNICATIONS.md`](COMMUNICATIONS.md) | Чаты, комментарии, уведомления. |
| [`CONFERENCING.md`](CONFERENCING.md) | Встроенная ВКС: WebRTC P2P/SFU, локальные TURN/STUN. |

## 📜 Стандарты, роли, качество

| Документ | Назначение |
|:---|:---|
| [`STANDARDS.md`](STANDARDS.md) | SCORM, xAPI, LTI, WCAG 2.2 AA, GDPR/CCPA. |
| [`RBAC.md`](RBAC.md) | Матрица ролей (Администратор, Инструктор, Ментор, Обучающийся) внутри тенантов. |
| [`DIAGNOSTICS.md`](DIAGNOSTICS.md) | Сквозное структурированное логирование (`tracing`). |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | Правила full-stack Rust, запреты, управление памятью. |
| [`GOTCHAS.md`](GOTCHAS.md) | Журнал зафиксированных технических ловушек сборки и рантайма. |

## 🚀 Инфраструктура и процесс

| Документ | Назначение |
|:---|:---|
| [`DEPLOY.md`](DEPLOY.md) | Docker Compose, Ingress Nginx, CSP, Air-gapped On-Premise. |
| [`AGENTS.md`](AGENTS.md) | Инструкции для AI-агентов (Claude Code, Cursor, Cline, Codex, Copilot). |
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | Стандарты коммитов и ветвления. |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Журнал изменений (заполняется по Conventional Commits). |