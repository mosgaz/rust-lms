# Текущий статус реализации системы (Feature Status Matrix)

Этот файл динамически обновляется AI-агентами после завершения каждого таска. Изменение статусов дублируется записью в `CHANGELOG.md`.

**Текущий этап проекта:** проектирование и разработка документации.
**Кодовая база:** начальная стадия.

## Легенда статусов

*   🔴 **НЕ СДЕЛАНО** — функционал отсутствует, контракты не объявлены.
*   🟡 **ЗАГЛУШКА** — объявлены интерфейсы/типы, но методы возвращают дефолтные mock-данные.
*   🟢 **ГОТОВО** — бизнес-логика написана, покрыта тестами ≥80%, RLS-проверки активны.
*   ⚪ **ПЛАН** — зафиксировано в спецификации, реализация не начата и не декомпозирована на таски.

## Крейты Cargo Workspace

Актуальный состав воркспейса (см. [`specs/STRUCTURE.md`](STRUCTURE.md)):

| Крейт | Назначение |
|:---|:---|
| `shared` | Плоские DTO, сущности, контракты обмена, xAPI JSON-LD. |
| `ui` | Атомарные компоненты, дизайн-система, Tailwind, a11y (WCAG 2.2 AA), i18n (Fluent, RTL). |
| `api` | Бизнес-логика, PostgreSQL + RLS, LRS, SCIM, ETL, license, Calculations Engine. |
| `client` | Leptos-приложение: `website`, `student`, `cpanel`, PWA, i18n. |
| `server` | Точка входа Axum, пулы СУБД, раздача WASM, планировщики Tokio, cron retention, валидация лицензии. |

## Типы строк

*   **Проектирование** — спецификация / регламент / контракт.
*   **Реализация** — код в соответствующем крейте.
*   **План** — направление из [`specs/ROADMAP.md`](ROADMAP.md).

---

## 📐 Проектирование и документация

| Артефакт | Статус | Ответственный | Примечания |
|:---|:---:|:---|:---|
| [`specs/README.md`](README.md) | 🟢 | — | Разводящая страница документации. |
| [`specs/SPECIFICATION.md`](SPECIFICATION.md) | 🟢 | — | Бизнес-концепция, иерархия программ, версионирование, retention, роли. |
| [`specs/ARCHITECTURE.md`](ARCHITECTURE.md) | 🟢 | — | Сводный ADD: RLS, Open API, LRS, плагины, ETL. |
| [`specs/NFR.md`](NFR.md) | 🟢 | — | SLA, RTO/RPO, concurrency, latency budgets, лимиты, Sizing Guide, retention трейсов. |
| [`specs/STRUCTURE.md`](STRUCTURE.md) | 🟢 | — | Карта папок, Dependency Rules, состав крейтов. |
| [`specs/DB_SCHEMA.md`](DB_SCHEMA.md) | 🟢 | — | Таблицы, RLS, версионирование, i18n, retention, LRS. |
| [`specs/MIGRATIONS.md`](MIGRATIONS.md) | 🟢 | — | Регламент на базе `sqlx` + Runbook для администратора On-Premise. |
| [`specs/OPEN_API.md`](OPEN_API.md) | 🟢 | — | REST/GraphQL, Opaque-токены, SCIM 2.0, signed-url, вебхуки, версионирование. |
| [`specs/OFFLINE_SYNC.md`](OFFLINE_SYNC.md) | 🟢 | — | IndexedDB, синхронизация, конфликты, iOS-лимиты, DRM, офлайн-шелл формы входа. |
| [`specs/PLUGIN.md`](PLUGIN.md) | 🟢 | — | Двухуровневый рантайм, FSM, подпись и kill switch. |
| [`specs/PLUGIN_DEVELOPMENT_TEMPLATE.md`](PLUGIN_DEVELOPMENT_TEMPLATE.md) | 🟢 | — | Шаблон ТЗ для внешних команд (включая a11y). |
| [`specs/DEPLOY.md`](DEPLOY.md) | 🟢 | — | Docker Compose, Ingress, CSP, COOP/COEP, Air-gapped, обновления, DR, сертификаты. |
| [`specs/LICENSING.md`](LICENSING.md) | 🟢 | — | Офлайн-лицензирование для коробочных поставок. |
| [`specs/STANDARDS.md`](STANDARDS.md) | 🟢 | — | SCORM, xAPI, LTI, SCIM 2.0, WCAG 2.2 AA, i18n, GDPR, retention, Conformance Testing. |
| [`specs/COMMUNICATIONS.md`](COMMUNICATIONS.md) | 🟢 | — | Чаты, комментарии, уведомления. |
| [`specs/CONFERENCING.md`](CONFERENCING.md) | 🟢 | — | WebRTC P2P/SFU, локальные TURN/STUN. |
| [`specs/ROADMAP.md`](ROADMAP.md) | 🟢 | — | Планы по аналитике, сертификации, биллингу, поиску, мобильному, аудиту, Conformance Testing. |
| [`specs/RBAC.md`](RBAC.md) | 🟢 | — | Матрица ролей и доступов. |
| [`specs/DIAGNOSTICS.md`](DIAGNOSTICS.md) | 🟢 | — | Логирование (`tracing`) и распределённый трейсинг (OpenTelemetry). |
| [`specs/GOTCHAS.md`](GOTCHAS.md) | 🟢 | — | Журнал технических ловушек. |
| [`specs/CODING_STANDARDS.md`](CODING_STANDARDS.md) | 🟢 | — | Правила full-stack Rust, RLS, запреты. |
| [`specs/AGENTS.md`](AGENTS.md) | 🟢 | — | Инструкции для AI-агентов. |
| [`specs/decisions/README.md`](decisions/README.md) | 🟢 | — | Реестр ADR, точка входа. |
| [`specs/decisions/2026.09.28-0001.md`](decisions/2026.09.28-0001.md) | 🟢 | — | ADR: RLS вместо схем-per-tenant. |
| [`specs/decisions/2026.09.28-0002.md`](decisions/2026.09.28-0002.md) | 🟢 | — | ADR: иммутабельный xAPI в LRS. |
| [`specs/decisions/2026.09.29-0003.md`](decisions/2026.09.29-0003.md) | 🟢 | — | ADR: подпись и kill switch для WASM-плагинов. |
| [`specs/decisions/2026.09.29-0004.md`](decisions/2026.09.29-0004.md) | 🟢 | — | ADR: Application-Level Encryption. |
| [`specs/decisions/2026.09.29-0005.md`](decisions/2026.09.29-0005.md) | 🟢 | — | ADR: Data Residency. |
| [`specs/decisions/2026.09.29-0006.md`](decisions/2026.09.29-0006.md) | 🟢 | — | ADR: выбор OTel backend для SaaS. |
| [`specs/decisions/2026.09.29-0007.md`](decisions/2026.09.29-0007.md) | 🟢 | — | ADR: операционный регламент deprecation API. |
| [`specs/decisions/2026.09.29-0008.md`](decisions/2026.09.29-0008.md) | 🟢 | — | ADR: формат и enforcement лицензионного ключа. |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | 🟢 | — | Коммиты, ветвление, Conventional Commits. |
| [`CHANGELOG.md`](../CHANGELOG.md) | 🟢 | — | Журнал изменений. |

---

## 📊 Матрица слоёв и фич (реализация)

| Компонент / Фича | Тип | Статус | Крейт-ответственный | Примечания / Ссылка на ADR |
|:---|:---:|:---:|:---|:---|
| **Мультиарендность (Strict Multi-tenancy)** | Реализация | 🔴 | `api` | Ожидает реализации RLS-интерцептора сессий. ADR: [`2026.09.28-0001.md`](decisions/2026.09.28-0001.md). |
| **Динамический Provisioning тенантов** | Реализация | 🔴 | `api` | Эндпоинт `POST /api/v1/internal/tenants`. |
| **Слой открытых токенов и API Keys** | Реализация | 🔴 | `api` | SHA-256 хэширование Opaque-ключей в БД. |
| **Конвейер кастомного импорта (ETL)** | Реализация | 🔴 | `api` | Потоковый парсинг CSV/XLSX чанками. |
| **SCIM 2.0 (Users/Groups)** | Реализация | 🔴 | `api` | Real-time provisioning из HRIS/HRM. См. [`OPEN_API.md`](OPEN_API.md) §3.5. |
| **Иерархия (Programs / Courses / Batches)** | Реализация | 🔴 | `shared` | Объявление плоских DTO-контрактов. |
| **Content Versioning & Cohort Pinning** | Реализация | 🔴 | `api` | `version`, `course_versions`, `batches.content_version`. |
| **Движок LRS (Аналитика xAPI)** | Реализация | 🔴 | `api` | Инвариантный слой TimescaleDB/ClickHouse. ADR: [`2026.09.28-0002.md`](decisions/2026.09.28-0002.md). |
| **Offline-First PWA (IndexedDB Queue)** | Реализация | 🔴 | `client` | Сервис-воркер и транзакционный буфер. ADR: [`2026.09.28-0002.md`](decisions/2026.09.28-0002.md). |
| **Ограничения iOS PWA (деградация)** | Реализация | 🔴 | `client` | Лимиты IndexedDB, отсутствие Background Sync. См. [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) §1.3. |
| **Signed URLs для медиа** | Реализация | 🔴 | `api` / `client` | HMAC-SHA256 + TTL; см. [`OPEN_API.md`](OPEN_API.md) §3.6. |
| **DRM (Widevine / FairPlay / PlayReady)** | Реализация | 🔴 | `api` / `client` | Опционально для Enterprise-тенантов. |
| **Рантайм плагинов Контур А (iframe)** | Реализация | 🔴 | `server` | Мост postMessage и CSP-изоляция шлюза. |
| **Рантайм плагинов Контур Б (WASM)** | Реализация | 🔴 | `server` | Zero-copy / copy-minimized биндинги. Подпись и kill switch — ADR: [`2026.09.29-0003.md`](decisions/2026.09.29-0003.md). |
| **Подпись и kill switch WASM-плагинов** | Реализация | 🔴 | `api` / `client` | Ed25519 + Revocation List. ADR: [`2026.09.29-0003.md`](decisions/2026.09.29-0003.md). |
| **Автоматическая сертификация** | Реализация | 🔴 | `server` | Фоновые воркеры (Tokio); Calculations Engine — часть `api`. |
| **Импорт SCORM 1.2 / 2004** | Реализация | 🔴 | `api` | Два режима: конвертация / runtime. См. [`STANDARDS.md`](STANDARDS.md) §SCORM. |
| **LTI 1.3 (Consumer + Provider)** | Реализация | 🔴 | `api` | Требования — в [`STANDARDS.md`](STANDARDS.md). |
| **Conformance Testing (базовый CLI)** | Реализация | 🔴 | `api` / `server` | CLI `rust-lms-conformance` + опциональный сервис. См. [`STANDARDS.md`](STANDARDS.md) §«Conformance Testing». |
| **Доступность WCAG 2.2 AA** | Реализация | 🔴 | `ui` | Требования — в [`STANDARDS.md`](STANDARDS.md); axe-core в CI. |
| **Локализация (i18n / l10n)** | Реализация | 🔴 | `ui` / `client` | Fluent, `*_i18n` поля, RTL. См. [`STANDARDS.md`](STANDARDS.md) §«Локализация». |
| **Retention Policies (ILM)** | Реализация | 🔴 | `server` | Cron-воркеры + таблица `retention_policies`. См. [`STANDARDS.md`](STANDARDS.md) §«Политики удержания данных». |
| **Application-Level Encryption (ALE)** | Реализация | 🔴 | `api` | Envelope encryption, KMS/Vault. ADR: [`2026.09.29-0004.md`](decisions/2026.09.29-0004.md). |
| **Data Residency (фиксация региона)** | Реализация | 🔴 | `api` / `server` | Размещение по регионам, запрет трансграничной передачи. ADR: [`2026.09.29-0005.md`](decisions/2026.09.29-0005.md). |
| **OpenTelemetry (SDK + OTLP + span attrs)** | Реализация | 🔴 | `api` / `server` / `client` | См. [`DIAGNOSTICS.md`](DIAGNOSTICS.md) §4. |
| **Подсистема лицензирования** | Реализация | 🔴 | `api` / `server` | Ed25519-ключ, три типа binding, soft/hard enforcement. ADR: [`2026.09.29-0008.md`](decisions/2026.09.29-0008.md). |
| **Чаты (личные / групповые / курс / задание)** | Реализация | 🔴 | `api` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **Комментарии к контенту (ветки)** | Реализация | 🔴 | `api` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **Уведомления (in-app / Email / Telegram / Webhook / Push)** | Реализация | 🔴 | `server` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **ВКС: P2P (Mesh) для 1-to-1** | Реализация | 🔴 | `client` | Требования — в [`CONFERENCING.md`](CONFERENCING.md). |
| **ВКС: SFU (Mediasoup/Janus) для групп** | Реализация | 🔴 | `api` | Требования — в [`CONFERENCING.md`](CONFERENCING.md). |
| **ВКС: Whiteboard, шеринг, опросы** | Реализация | 🔴 | `ui` | Требования — в [`CONFERENCING.md`](CONFERENCING.md). |
| **ВКС: локальные TURN/STUN (Air-gapped)** | Реализация | 🔴 | `server` | Требования — в [`CONFERENCING.md`](CONFERENCING.md) и [`DEPLOY.md`](DEPLOY.md). |

---

## 🗺️ Плановые направления (без декомпозиции на таски)

Все направления зафиксированы в [`specs/ROADMAP.md`](ROADMAP.md). Статус ⚪ — реализация не начата и не декомпозирована.

| Направление | Статус | Примечания |
|:---|:---:|:---|
| Аналитика обучения (дашборды, отчёты, алерты) | ⚪ | Сбор данных покрыт LRS; слой осмысления — в плане. |
| Сертификация и валидация (Open Badges / Blockcerts) | ⚪ | Выдача и верификация сертификатов; см. [`SPECIFICATION.md`](SPECIFICATION.md) §4. |
| Платежи и биллинг (подписки, счета, налоги) | ⚪ | Интеграция с провайдерами. |
| Поиск (Elasticsearch/Meilisearch) | ⚪ | Полнотекстовый поиск по контенту. |
| Рекомендации курсов и материалов | ⚪ | На основе истории и профиля. |
| Вебинары с записью | ⚪ | Поверх встроенной ВКС. |
| Мобильное приложение (нативное) | ⚪ | Приоритет для iOS-сегмента (см. [`OFFLINE_SYNC.md`](OFFLINE_SYNC.md) §1.3). Технологии будут определены позже. |
| Аудит и compliance (SIEM, retention) | ⚪ | Расширение [`specs/STANDARDS.md`](STANDARDS.md). |
| Conformance Testing (расширение для On-Premise) | ⚪ | Полный ADL SCORM Test Suite, cmi5, LTI 1.3, SCIM 2.0; UI в `cpanel`. См. [`ROADMAP.md`](ROADMAP.md) §«Планируемые направления». |