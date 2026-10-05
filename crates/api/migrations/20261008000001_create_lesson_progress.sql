-- crates/api/migrations/20261008000001_create_lesson_progress.sql
-- Прогресс обучения: отслеживание прогресса по урокам и автоматическое завершение курсов.
--
-- Архитектура:
-- - `lesson_progress` — прогресс студента по конкретному уроку
-- - `course_enrollments.completed_lessons_weight` — денормализованный вес завершённых уроков
-- - `nodes.is_archived` — мягкое удаление уроков (архивация)
--
-- Все таблицы tenant-scoped с RLS.

-- Шаг 1: Добавление поля is_archived в таблицу nodes (для мягкого удаления)
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS is_archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ;

-- Partial index для активных (неархивных) узлов — 99% запросов
CREATE INDEX IF NOT EXISTS idx_nodes_active ON nodes(tenant_id, id) WHERE is_archived = FALSE;

COMMENT ON COLUMN nodes.is_archived IS 'Мягкое удаление урока (архивация). Архивные уроки исключаются из пересчёта прогресса';
COMMENT ON COLUMN nodes.archived_at IS 'Когда урок был архивирован';

-- Шаг 2: Добавление поля completed_lessons_weight в таблицу course_enrollments
-- Индекс на это поле НЕ создаём: оно часто обновляется, но редко используется в WHERE
ALTER TABLE course_enrollments ADD COLUMN IF NOT EXISTS completed_lessons_weight NUMERIC(10, 4) NOT NULL DEFAULT 0.0000;

COMMENT ON COLUMN course_enrollments.completed_lessons_weight IS 'Суммарный вес завершённых уроков (для инкрементального пересчёта прогресса)';

-- Шаг 3: Создание таблицы lesson_progress
CREATE TABLE IF NOT EXISTS lesson_progress (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'not_started',
    score NUMERIC(5, 4),
    passed BOOLEAN,
    time_spent_seconds INTEGER NOT NULL DEFAULT 0,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_position INTEGER NOT NULL DEFAULT 0,
    completed_at TIMESTAMPTZ,
    client_modified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (user_id, node_id),
    CONSTRAINT chk_lesson_progress_status CHECK (
        status IN ('not_started', 'in_progress', 'completed')
    ),
    CONSTRAINT chk_lesson_progress_score CHECK (
        score IS NULL OR (score >= 0.0 AND score <= 1.0)
    ),
    CONSTRAINT chk_lesson_progress_time CHECK (
        time_spent_seconds >= 0
    ),
    CONSTRAINT chk_lesson_progress_attempts CHECK (
        attempt_count >= 0
    ),
    CONSTRAINT chk_lesson_progress_position CHECK (
        last_position >= 0
    ),
    -- passed и score логически связаны: оба NULL или оба NOT NULL
    CONSTRAINT chk_lesson_progress_passed_score_consistency CHECK (
        (passed IS NULL AND score IS NULL) OR
        (passed IS NOT NULL AND score IS NOT NULL)
    )
);

ALTER TABLE lesson_progress ENABLE ROW LEVEL SECURITY;

-- Оптимизированная RLS-политика: cast TEXT → UUID один раз (не на каждой строке)
CREATE POLICY lesson_progress_tenant_isolation_policy ON lesson_progress
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

CREATE INDEX IF NOT EXISTS idx_lesson_progress_tenant_id ON lesson_progress(tenant_id);
CREATE INDEX IF NOT EXISTS idx_lesson_progress_user_id ON lesson_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_lesson_progress_node_id ON lesson_progress(node_id);
CREATE INDEX IF NOT EXISTS idx_lesson_progress_status ON lesson_progress(status);
CREATE INDEX IF NOT EXISTS idx_lesson_progress_user_node ON lesson_progress(user_id, node_id);

-- Partial index для завершённых уроков — частый кейс для отчётов
CREATE INDEX IF NOT EXISTS idx_lesson_progress_completed 
ON lesson_progress(user_id, completed_at) 
WHERE status = 'completed';

COMMENT ON TABLE lesson_progress IS 'Прогресс студента по конкретному уроку';
COMMENT ON COLUMN lesson_progress.status IS 'Статус урока: not_started, in_progress, completed';
COMMENT ON COLUMN lesson_progress.score IS 'Балл за тест (0.0–1.0), NULL для нетестовых уроков';
COMMENT ON COLUMN lesson_progress.passed IS 'Сдан ли тест (вычисляется сервером на основе score и passing_score)';
COMMENT ON COLUMN lesson_progress.time_spent_seconds IS 'Общее время в уроке (секунды)';
COMMENT ON COLUMN lesson_progress.attempt_count IS 'Количество попыток (увеличивается только для тестов при completed)';
COMMENT ON COLUMN lesson_progress.last_position IS 'Позиция в медиа (секунды) для возобновления';
COMMENT ON COLUMN lesson_progress.completed_at IS 'Когда урок завершён (status = completed)';
COMMENT ON COLUMN lesson_progress.client_modified_at IS 'Время последнего изменения на клиенте (зарезервировано для Этапа 11)';

-- Шаг 4: Триггер для автоматического обновления updated_at
CREATE OR REPLACE FUNCTION update_lesson_progress_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_lesson_progress_updated_at
    BEFORE UPDATE ON lesson_progress
    FOR EACH ROW
    EXECUTE FUNCTION update_lesson_progress_updated_at();

-- Шаг 5: Комментарии для документации
COMMENT ON CONSTRAINT chk_lesson_progress_score ON lesson_progress IS 'Score от 0.0 до 1.0 или NULL';
COMMENT ON CONSTRAINT chk_lesson_progress_time ON lesson_progress IS 'Время не может быть отрицательным';
COMMENT ON CONSTRAINT chk_lesson_progress_attempts ON lesson_progress IS 'Количество попыток не может быть отрицательным';
COMMENT ON CONSTRAINT chk_lesson_progress_position ON lesson_progress IS 'Позиция в медиа не может быть отрицательной';
COMMENT ON CONSTRAINT chk_lesson_progress_passed_score_consistency ON lesson_progress IS 'passed и score должны быть оба NULL или оба NOT NULL';