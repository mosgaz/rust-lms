#### Таблица: course_enrollments (Защищена RLS)

Таблица связей для индивидуального зачисления пользователей на курсы (вне потоков).

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK → users.id ON DELETE CASCADE).
* `course_id`: UUID (FK → courses.id ON DELETE CASCADE).
* `status`: VARCHAR(32) (CHECK: `'active'`, `'completed'`, `'dropped'`).
* `progress`: NUMERIC(5, 4) (NOT NULL, DEFAULT 0.0000). Общий прогресс курса (0.0–1.0).
* `completed_lessons_weight`: NUMERIC(10, 4) (NOT NULL, DEFAULT 0.0000). Суммарный вес завершённых уроков (для O(1)-пересчёта прогресса).
* `completed_at`: TIMESTAMPTZ (NULLable). Когда курс был завершён.
* `enrolled_at`: TIMESTAMPTZ (NOT NULL, DEFAULT NOW()).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:** Уникальный индекс `UNIQUE (tenant_id, user_id, course_id)`.
* **RLS Политика:** `course_enrollment_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

> **Инкрементальный пересчёт прогресса.** Поле `completed_lessons_weight` обновляется транзакционно при каждом upsert в `lesson_progress` со `status = 'completed'`. Это позволяет пересчитывать `progress` за O(1) вместо агрегатного `SUM(weight)` по всем урокам курса.

#### Таблица: lesson_progress (Защищена RLS)

Отслеживание прогресса студента по конкретному уроку.

* `id`: UUID (Primary Key).
* `tenant_id`: UUID (FK → tenants.id ON DELETE CASCADE).
* `user_id`: UUID (FK → users.id ON DELETE CASCADE).
* `node_id`: UUID (FK → nodes.id ON DELETE CASCADE). Узел типа `lesson`.
* `status`: VARCHAR(32) (CHECK: `'not_started'`, `'in_progress'`, `'completed'`).
* `score`: NUMERIC(5, 4) (NULLable). Балл за тест (0.0–1.0).
* `passed`: BOOLEAN (NULLable). Сдан ли тест (вычисляется сервером на основе `score >= passing_score`).
* `time_spent_seconds`: INTEGER (NOT NULL, DEFAULT 0). Общее время в уроке.
* `attempt_count`: INTEGER (NOT NULL, DEFAULT 0). Количество попыток (увеличивается только для тестов при `status = 'completed'`).
* `last_position`: INTEGER (NOT NULL, DEFAULT 0). Позиция в медиа (секунды) для возобновления.
* `completed_at`: TIMESTAMPTZ (NULLable). Когда урок завершён.
* `client_modified_at`: TIMESTAMPTZ (NULLable). Зарезервировано для синхронизации PWA (Этап 11).
* `created_at` / `updated_at`: TIMESTAMPTZ.
* **Ограничения:**
  * Уникальный индекс `UNIQUE (user_id, node_id)`.
  * CHECK: `score >= 0 AND score <= 1` (если не NULL).
  * CHECK: `time_spent_seconds >= 0`, `attempt_count >= 0`, `last_position >= 0`.
* **Индексы:**
  * `idx_lesson_progress_tenant_id` (для RLS).
  * `idx_lesson_progress_user_id` (быстрый поиск прогресса студента).
  * `idx_lesson_progress_node_id` (поиск по уроку).
  * `idx_lesson_progress_status` (фильтрация по статусу).
  * `idx_lesson_progress_completed` (partial index для завершённых уроков).
* **RLS Политика:** `lesson_progress_tenant_isolation_policy` (фильтрация по `tenant_id = current_setting('app.current_tenant_id', true)`).

> **Сервер вычисляет `passed`.** Клиент присылает только `score`, сервер вычисляет `passed = (score >= passing_score)` на основе `nodes.metadata.quiz.passing_score`. Это защита от подделки результатов тестов.

> **Архивные уроки.** Уроки с `nodes.is_archived = true` исключаются из пересчёта прогресса курса. Попытка обновить прогресс архивного урока возвращает `410 Gone`.

> **Идемпотентность.** Повторный запрос с теми же данными не перезаписывает `completed_at` — сохраняется время первого завершения.

> **Триггер денормализации веса.** Поле `courses.metadata.total_weight` автоматически пересчитывается триггером `recalculate_course_total_weight()` при любых изменениях `nodes` (создание, удаление, архивация, изменение веса). Это исключает агрегатный запрос `SUM(weight)` при каждом upsert.