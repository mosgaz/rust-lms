-- crates/api/migrations/20261007000001_create_batches_and_enrollments.sql
-- Потоки (Batches) и зачисления (Enrollments) для группового и индивидуального обучения.
--
-- Архитектура:
-- - `batches` — потоки (группы студентов, проходящих курсы вместе)
-- - `batch_courses` — связь Batch ↔ Course (какие курсы входят в поток)
-- - `batch_enrollments` — связь User ↔ Batch с ролью (student/instructor/tutor/observer)
-- - `course_enrollments` — связь User ↔ Course для self-paced курсов
--
-- Все таблицы tenant-scoped с RLS.

-- Шаг 1: Переименование certification_rules → completion_criteria в таблице courses
ALTER TABLE courses RENAME COLUMN certification_rules TO completion_criteria;

COMMENT ON COLUMN courses.completion_criteria IS 'Настраиваемые критерии завершения курса (JSONB): {"mode": "all_of", "rules": [{"type": "min_progress", "value": 0.8}]}';

-- Шаг 2: Создание таблицы batches (потоки)
CREATE TABLE IF NOT EXISTS batches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    enrollment_deadline TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_batches_status CHECK (
        status IN ('draft', 'active', 'archived', 'completed')
    ),
    CONSTRAINT chk_batches_dates CHECK (
        (start_date IS NULL OR end_date IS NULL OR start_date <= end_date)
    )
);

ALTER TABLE batches ENABLE ROW LEVEL SECURITY;

CREATE POLICY batches_tenant_isolation_policy ON batches
    FOR ALL
    USING (tenant_id::text = current_setting('app.current_tenant_id', true));

CREATE INDEX IF NOT EXISTS idx_batches_tenant_id ON batches(tenant_id);
CREATE INDEX IF NOT EXISTS idx_batches_status ON batches(status);

COMMENT ON TABLE batches IS 'Потоки (группы студентов, проходящих курсы вместе)';
COMMENT ON COLUMN batches.status IS 'Статус потока: draft, active, archived, completed';

-- Шаг 3: Создание таблицы batch_courses (связь Batch ↔ Course)
CREATE TABLE IF NOT EXISTS batch_courses (
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    course_id UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (batch_id, course_id)
);

CREATE INDEX IF NOT EXISTS idx_batch_courses_batch_id ON batch_courses(batch_id);
CREATE INDEX IF NOT EXISTS idx_batch_courses_course_id ON batch_courses(course_id);

COMMENT ON TABLE batch_courses IS 'Связь Batch ↔ Course (какие курсы входят в поток)';

-- Шаг 4: Создание таблицы batch_enrollments (связь User ↔ Batch с ролью)
CREATE TABLE IF NOT EXISTS batch_enrollments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'student',
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'active',

    UNIQUE (batch_id, user_id),
    CONSTRAINT chk_batch_enrollments_role CHECK (
        role IN ('student', 'instructor', 'tutor', 'observer')
    ),
    CONSTRAINT chk_batch_enrollments_status CHECK (
        status IN ('active', 'completed', 'dropped')
    )
);

ALTER TABLE batch_enrollments ENABLE ROW LEVEL SECURITY;

CREATE POLICY batch_enrollments_tenant_isolation_policy ON batch_enrollments
    FOR ALL
    USING (
        batch_id IN (
            SELECT id FROM batches WHERE tenant_id::text = current_setting('app.current_tenant_id', true)
        )
    );

CREATE INDEX IF NOT EXISTS idx_batch_enrollments_batch_id ON batch_enrollments(batch_id);
CREATE INDEX IF NOT EXISTS idx_batch_enrollments_user_id ON batch_enrollments(user_id);
CREATE INDEX IF NOT EXISTS idx_batch_enrollments_status ON batch_enrollments(status);

COMMENT ON TABLE batch_enrollments IS 'Связь User ↔ Batch с ролью (student/instructor/tutor/observer)';
COMMENT ON COLUMN batch_enrollments.role IS 'Роль участника потока: student, instructor, tutor, observer';
COMMENT ON COLUMN batch_enrollments.status IS 'Статус зачисления: active, completed, dropped (soft delete)';

-- Шаг 5: Создание таблицы course_enrollments (связь User ↔ Course для self-paced)
CREATE TABLE IF NOT EXISTS course_enrollments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    course_id UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    progress NUMERIC(5, 4) DEFAULT 0.0000,
    status VARCHAR(32) NOT NULL DEFAULT 'active',

    UNIQUE (course_id, user_id),
    CONSTRAINT chk_course_enrollments_progress CHECK (
        progress >= 0.0 AND progress <= 1.0
    ),
    CONSTRAINT chk_course_enrollments_status CHECK (
        status IN ('active', 'completed', 'dropped')
    )
);

ALTER TABLE course_enrollments ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_enrollments_tenant_isolation_policy ON course_enrollments
    FOR ALL
    USING (
        course_id IN (
            SELECT id FROM courses WHERE tenant_id::text = current_setting('app.current_tenant_id', true)
        )
    );

CREATE INDEX IF NOT EXISTS idx_course_enrollments_course_id ON course_enrollments(course_id);
CREATE INDEX IF NOT EXISTS idx_course_enrollments_user_id ON course_enrollments(user_id);
CREATE INDEX IF NOT EXISTS idx_course_enrollments_status ON course_enrollments(status);

COMMENT ON TABLE course_enrollments IS 'Связь User ↔ Course для self-paced курсов';
COMMENT ON COLUMN course_enrollments.progress IS 'Прогресс курса (0.0–1.0), обновляется автоматически на Этапе 9';
COMMENT ON COLUMN course_enrollments.status IS 'Статус зачисления: active, completed, dropped (soft delete)';

-- Шаг 6: Комментарии для документации
COMMENT ON CONSTRAINT chk_batches_dates ON batches IS 'Проверка: start_date <= end_date';
COMMENT ON CONSTRAINT chk_batch_enrollments_role ON batch_enrollments IS 'Роли: student, instructor, tutor, observer (observer добавлен для руководителей/HR)';
COMMENT ON CONSTRAINT chk_course_enrollments_progress ON course_enrollments IS 'Прогресс от 0.0 до 1.0';