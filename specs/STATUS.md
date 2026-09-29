# Текущий статус реализации системы (Feature Status Matrix)

Этот файл динамически обновляется AI-агентами после завершения каждого таска. Изменение статусов дублируется записью в `CHANGELOG.md`.

**Текущий этап проекта:** проектирование и разработка документации.
**Кодовая база:** начальная стадия.

## Легенды статусов

Этот файл использует **три независимые легенды** — в зависимости от типа строки. Не путать: одна и та же иконка может означать разное в разных блоках.

### Легенда A — Проектирование и документация

*   🟢 **УТВЕРЖДЕНО** — спецификация выверена, согласована, готова к использованию как источник истины.
*   🟡 **ЧЕРНОВИК** — спецификация в работе, допускаются изменения.
*   🔴 **НЕ НАЧАТО** — спецификация отсутствует или обозначена только в навигации.

### Легенда B — Реализация

*   🟢 **ГОТОВО** — бизнес-логика написана, покрыта тестами ≥80%, RLS-проверки активны.
*   🟡 **ЗАГЛУШКА** — объявлены интерфейсы/типы, но методы возвращают дефолтные mock-данные.
*   🔴 **НЕ СДЕЛАНО** — функционал отсутствует, контракты не объявлены.

### Легенда C — Плановые направления

*   ⚪ **ПЛАН** — зафиксировано в спецификации, реализация не начата и не декомпозирована на таски.

## Крейты Cargo Workspace

Актуальный состав воркспейса (см. [`specs/STRUCTURE.md`](STRUCTURE.md)):

| Крейт | Назначение |
|:---|:---|
| `shared` | Плоские DTO, сущности, контракты обмена, xAPI JSON-LD. |
| `ui` | Атомарные компоненты, дизайн-система, Tailwind, a11y (WCAG 2.2 AA), i18n (Fluent, RTL). |
| `api` | Бизнес-логика, PostgreSQL + RLS, LRS, SCIM, ETL, license, feature flags, SBOM-сканирование, Calculations Engine. |
| `client` | Leptos-приложение: `website`, `student`, `cpanel`, PWA, i18n, кэш feature flags. |
| `server` | Точка входа Axum, пулы СУБД, раздача WASM, планировщики Tokio, cron retention, валидация лицензии, LISTEN/NOTIFY. |

---

## 📐 Проектирование и документация

> Легенда A: 🟢 УТВЕРЖДЕНО / 🟡 ЧЕРНОВИК / 🔴 НЕ НАЧАТО.

| Артефакт | Статус | Ответственный | Примечания |
|:---|:---:|:---|:---|
| [`specs/README.md`](README.md) | 🟢 | — | Разводящая страница документации. |
| [`specs/SPECIFICATION.md`](SPECIFICATION.md) | 🟢 | — | Бизнес-концепция, иерархия программ, версионирование, retention, роли, Feature Flags. |
| [`specs/ARCHITECTURE.md`](ARCHITECTURE.md) | 🟢 | — | Сводный ADD: RLS, Open API, LRS, плагины, ETL. |
| [`specs/NFR.md`](NFR.md) | 🟢 | — | SLA, RTO/RPO, concurrency, latency budgets, лимиты, Sizing Guide, retention трейсов. |
| [`specs/STRUCTURE.md`](STRUCTURE.md) | 🟢 | — | Карта папок, Dependency Rules, состав крейтов. |
| [`specs/DB_SCHEMA.md`](DB_SCHEMA.md) | 🟢 | — | Таблицы, RLS, версионирование, i18n, retention, LRS, feature flags, license, revoked JWT kids. |
| [`specs/MIGRATIONS.md`](MIGRATIONS.md) | 🟢 | — | Регламент на базе `sqlx` + Runbook для администратора On-Premise. |
| [`specs/OPEN_API.md`](OPEN_API.md) | 🟢 | — | REST/GraphQL, Opaque-токены, SCIM 2.0, signed-url, вебхуки, версионирование. |
| [`specs/OFFLINE_SYNC.md`](OFFLINE_SYNC.md) | 🟢 | — | IndexedDB, синхронизация, конфликты, iOS-лимиты, DRM, офлайн-шелл формы входа. |
| [`specs/PLUGIN.md`](PLUGIN.md) | 🟢 | — | Двухуровневый рантайм, FSM, подпись и kill switch, SBOM (ссылка на ADR-0010). |
| [`specs/PLUGIN_DEVELOPMENT_TEMPLATE.md`](PLUGIN_DEVELOPMENT_TEMPLATE.md) | 🟢 | — | Шаблон ТЗ для внешних команд (включая a11y). |
| [`specs/DEPLOY.md`](DEPLOY.md) | 🟢 | — | Docker Compose, Ingress, CSP, COOP/COEP, Air-gapped, обновления, DR, сертификаты, управление ключами, runbook JWT, Chaos Engineering. |
| [`specs/LICENSING.md`](LICENSING.md) | 🟢 | — | Офлайн-лицензирование для коробочных поставок. |
| [`specs/FEATURE_FLAGS.md`](FEATURE_FLAGS.md) | 🟢 | — | Управление функциональными флагами (глобальные + tenant overrides). |
| [`specs/STANDARDS.md`](STANDARDS.md) | 🟢 | — | SCORM, xAPI, LTI, SCIM 2.0, WCAG 2.2 AA, i18n, GDPR, retention, Conformance Testing, Data Portability. |
| [`specs/COMMUNICATIONS.md`](COMMUNICATIONS.md) | 🟢 | — | Чаты, комментарии, уведомления. |
| [`specs/CONFERENCING.md`](CONFERENCING.md) | 🟢 | — | WebRTC P2P/SFU, локальные TURN/STUN, ограничения MVP. |
| [`specs/ROADMAP.md`](ROADMAP.md) | 🟢 | — | Планы по аналитике, сертификации, биллингу, поиску, мобильному, аудиту, Conformance Testing, a11y, Data Portability, Feature Flags ext. |
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
| [`specs/decisions/2026.09.29-0009.md`](decisions/2026.09.29-0009.md) | 🟢 | — | ADR: архитектура Feature Flags. |
| [`specs/decisions/2026.09.29-0010.md`](decisions/2026.09.29-0010.md) | 🟢 | — | ADR: Supply Chain Security для WASM-плагинов. |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) | 🟢 | — | Коммиты, ветвление, Conventional Commits. |
| [`CHANGELOG.md`](../CHANGELOG.md) | 🟢 | — | Журнал изменений. |

---

## 📊 Матрица слоёв и фич (реализация)

> Легенда B: 🟢 ГОТОВО / 🟡 ЗАГЛУШКА / 🔴 НЕ СДЕЛАНО.

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
| **Рантайм плагинов Контур Б (WASM)** | Реализация | 🔴 | `server` | Zero-copy / copy-minimized биндинги. ADR: [`2026.09.29-0003.md`](decisions/2026.09.29-0003.md). |
| **Подпись и kill switch WASM-плагинов** | Реализация | 🔴 | `api` / `client` | Ed25519 + Revocation List. ADR: [`2026.09.29-0003.md`](decisions/2026.09.29-0003.md). |
| **Supply Chain Security для WASM (SBOM)** | Реализация | 🔴 | `api` / `server` | CycloneDX, `osv-scanner`, интеграция с Revocation List. ADR: [`2026.09.29-0010.md`](decisions/2026.09.29-0010.md). |
| **Автоматическая сертификация** | Реализация | 🔴 | `server` | Фоновые воркеры (Tokio); Calculations Engine — часть `api`. |
| **Импорт SCORM 1.2 / 2004** | Реализация | 🔴 | `api` | Режим 1 (сторонние) — без детального трекинга; Режим 1 (собственные) — `sendEvent`; Режим 2 — полный трекинг. См. [`STANDARDS.md`](STANDARDS.md) §SCORM. |
| **LTI 1.3 (Consumer + Provider)** | Реализация | 🔴 | `api` | Требования — в [`STANDARDS.md`](STANDARDS.md). |
| **Conformance Testing (базовый CLI)** | Реализация | 🔴 | `api` / `server` | CLI `rust-lms-conformance` + опциональный сервис. См. [`STANDARDS.md`](STANDARDS.md) §«Conformance Testing». |
| **Data Portability (базовый экспорт)** | Реализация | 🔴 | `api` | xAPI JSON-LD, сертификаты, GDPR-экспорт. Расширенные форматы (OneRoster, QTI) — в плане. См. [`STANDARDS.md`](STANDARDS.md) §«Data Portability». |
| **Доступность WCAG 2.2 AA** | Реализация | 🔴 | `ui` | Требования — в [`STANDARDS.md`](STANDARDS.md); axe-core в CI. |
| **Локализация (i18n / l10n)** | Реализация | 🔴 | `ui` / `client` | Fluent, `*_i18n` поля, RTL. См. [`STANDARDS.md`](STANDARDS.md) §«Локализация». |
| **Retention Policies (ILM)** | Реализация | 🔴 | `server` | Cron-воркеры + таблица `retention_policies`. См. [`STANDARDS.md`](STANDARDS.md) §«Политики удержания данных». |
| **Application-Level Encryption (ALE)** | Реализация | 🔴 | `api` | Envelope encryption, KMS/Vault. ADR: [`2026.09.29-0004.md`](decisions/2026.09.29-0004.md). |
| **Data Residency (фиксация региона)** | Реализация | 🔴 | `api` / `server` | Размещение по регионам, запрет трансграничной передачи. ADR: [`2026.09.29-0005.md`](decisions/2026.09.29-0005.md). |
| **OpenTelemetry (SDK + OTLP + span attrs)** | Реализация | 🔴 | `api` / `server` / `client` | См. [`DIAGNOSTICS.md`](DIAGNOSTICS.md) §4. |
| **Подсистема лицензирования** | Реализация | 🔴 | `api` / `server` | Ed25519-ключ, три типа binding, soft/hard enforcement. ADR: [`2026.09.29-0008.md`](decisions/2026.09.29-0008.md). |
| **Feature Flags (инфраструктура)** | Реализация | 🔴 | `api` / `server` | PostgreSQL + in-memory кэш + LISTEN/NOTIFY + polling 60 сек. ADR: [`2026.09.29-0009.md`](decisions/2026.09.29-0009.md). |
| **Управление криптоключами (Air-gapped)** | Реализация | 🔴 | `server` | Генерация, ротация, бэкап, аудит Ed25519/DEK/KEK. См. [`DEPLOY.md`](DEPLOY.md) §4.4. |
| **Runbook компрометации JWT** | Реализация | 🔴 | `api` / `server` | Таблица `revoked_jwt_kids`, вебхук `security.jwt_key_rotated`. См. [`DEPLOY.md`](DEPLOY.md) §4.5. |
| **Chaos Engineering (SaaS)** | Реализация | 🔴 | `server` | Ежеквартальные учения. См. [`DEPLOY.md`](DEPLOY.md) §4.6. |
| **Чаты (личные / групповые / курс / задание)** | Реализация | 🔴 | `api` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **Комментарии к контенту (ветки)** | Реализация | 🔴 | `api` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **Уведомления (in-app / Email / Telegram / Webhook / Push)** | Реализация | 🔴 | `server` | Требования — в [`COMMUNICATIONS.md`](COMMUNICATIONS.md). |
| **ВКС: P2P (Mesh) для 1-to-1** | Реализация | 🔴 | `client` | Fallback на SFU+TURN при неудачном ICE за 5 сек. См. [`CONFERENCING.md`](CONFERENCING.md) §1.1. |
| **ВКС: SFU (Mediasoup/Janus) для групп** | Реализация | 🔴 | `api` | Разворачивается по умолчанию в Air-gapped. См. [`CONFERENCING.md`](CONFERENCING.md) §3. |
| **ВКС: Whiteboard, шеринг, опросы** | Реализация | 🔴 | `ui` | Требования — в [`CONFERENCING.md`](CONFERENCING.md) §1.2. |
| **ВКС: локальные TURN/STUN (Air-gapped)** | Реализация | 🔴 | `server` | Разворачиваются по умолчанию. См. [`CONFERENCING.md`](CONFERENCING.md) §4. |

---

## 🗺️ Плановые направления (без декомпозиции на таски)

> Легенда C: ⚪ ПЛАН.

Все направления зафиксированы в [`specs/ROADMAP.md`](ROADMAP.md).

| Направление | Статус | Примечания |
|:---|:---:|:---|
| Аналитика обучения (дашборды, отчёты, алерты) | ⚪ | Сбор данных покрыт LRS; слой осмысления — в плане. |
| Сертификация и валидация (Open Badges / Blockcerts) | ⚪ | Выдача и верификация сертификатов; см. [`SPECIFICATION.md`](SPECIFICATION.md) §4. |
| Платежи и биллинг (подписки, счета, налоги) | ⚪ | Интеграция с провайдерами. |
| Поиск (Elasticsearch/Meilisearch) | ⚪ | Полнотекстовый поиск по контенту. |
| Рекомендации курсов и материалов | ⚪ | На основе истории и профиля. |
| Вебинары с записью | ⚪ | Поверх встроенной ВКС. |
| Мобильное приложение — MVP | ⚪ | PWA с ограниченным офлайном на iOS; нативное приложение — в 1.x. См. [`ROADMAP.md`](ROADMAP.md) §«Мобильное приложение». |
| Мобильное приложение — v1.x (нативное, iOS) | ⚪ | Закрывает сценарий полного офлайна для iOS. Сроки TBD. |
| Аудит и compliance (SIEM, retention) | ⚪ | Расширение [`specs/STANDARDS.md`](STANDARDS.md). |
| Conformance Testing (расширение для On-Premise) | ⚪ | Полный ADL SCORM Test Suite, cmi5, LTI 1.3, SCIM 2.0; UI в `cpanel`. См. [`ROADMAP.md`](ROADMAP.md) §«Conformance Testing». |
| Расширенное a11y-тестирование | ⚪ | Playwright + NVDA/VoiceOver, внешний аудит WCAG 2.2 AA. См. [`ROADMAP.md`](ROADMAP.md) §«Расширенное a11y-тестирование». |
| Data Portability — расширенные форматы (OneRoster, QTI 2.1, LTI AGS) | ⚪ | См. [`ROADMAP.md`](ROADMAP.md) §«Data Portability». |
| Feature Flags — расширения (A/B-тестирование, auto-rollout, UI в `cpanel`) | ⚪ | См. [`ROADMAP.md`](ROADMAP.md) §«Feature Flags — расширения». |
| SCORM 2004 Sequencing & Navigation | ⚪ | Расширение для Enterprise-тенантов. |
| Динамическое переключение P2P → SFU в активной сессии | ⚪ | Для v1.x ВКС. См. [`CONFERENCING.md`](CONFERENCING.md) §8. |
| GraphQL — полноценная спецификация схемы | ⚪ | Вынесено за MVP. |
| Deprecation версий API — операционный регламент | ⚪ | Политика в [`OPEN_API.md`](OPEN_API.md) §5; регламент — в плане. |