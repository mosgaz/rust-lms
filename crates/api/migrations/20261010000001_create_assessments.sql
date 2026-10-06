-- crates/api/migrations/20261010000001_create_assessments.sql
-- Миграция для Этапа 10: Assessments Engine.
-- Создаёт таблицы вопросов, попыток и ответов с RLS-изоляцией.

-- ============================================================
-- 1. Таблица questions
-- ============================================================

CREATE TABLE questions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    course_id       UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,

    title           TEXT NOT NULL CHECK (char_length(title) > 0),
    description     TEXT,
    question_type   VARCHAR(32) NOT NULL CHECK (question_type IN (
                        'multiple_choice', 'true_false', 'short_answer', 'long_answer'
                    )),

    -- Варианты ответов для multiple_choice (JSONB-массив)
    -- Пример: [{"text": "...", "is_correct": true}, ...]
    options         JSONB,

    -- Правильный ответ для short_answer/true_false
    correct_answer  TEXT,

    -- Баллы за правильный ответ
    points          INTEGER NOT NULL DEFAULT 1 CHECK (points >= 0),

    -- Порядок вопроса в тесте (1-based)
    "order"         INTEGER NOT NULL CHECK ("order" >= 1),

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Индексы
CREATE INDEX idx_questions_tenant_id ON questions(tenant_id);
CREATE INDEX idx_questions_course_id ON questions(course_id);
CREATE INDEX idx_questions_order ON questions(course_id, "order");

-- Уникальность порядка вопросов в рамках курса
CREATE UNIQUE INDEX idx_questions_course_order ON questions(course_id, "order");

-- RLS
ALTER TABLE questions ENABLE ROW LEVEL SECURITY;

CREATE POLICY questions_tenant_isolation ON questions
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);

-- Триггер для автообновления updated_at
CREATE TRIGGER trg_questions_updated_at
    BEFORE UPDATE ON questions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE questions IS
    'Вопросы для тестов. question_type определяет формат: '
    'multiple_choice (используется options), true_false (correct_answer = "true"/"false"), '
    'short_answer/long_answer (correct_answer — эталонный текст).';

-- ============================================================
-- 2. Таблица attempts
-- ============================================================

CREATE TABLE attempts (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id               UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    course_id             UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,

    status                VARCHAR(32) NOT NULL DEFAULT 'in_progress'
                          CHECK (status IN (
                              'in_progress', 'completed', 'timed_out', 'abandoned'
                          )),

    started_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ,

    -- Итоговый балл (0.0–1.0). NULL до завершения попытки.
    score                 NUMERIC(5, 4) CHECK (score >= 0.0 AND score <= 1.0),
    passed                BOOLEAN,

    -- Лимит времени в секундах (NULL = без лимита)
    time_limit_seconds    INTEGER CHECK (time_limit_seconds IS NULL OR time_limit_seconds > 0),

    -- Фактически затраченное время
    time_spent_seconds    INTEGER NOT NULL DEFAULT 0 CHECK (time_spent_seconds >= 0),

    -- Номер попытки этого студента по этому курсу (монотонно растёт)
    attempt_number        INTEGER NOT NULL CHECK (attempt_number >= 1),

    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Индексы
CREATE INDEX idx_attempts_tenant_id ON attempts(tenant_id);
CREATE INDEX idx_attempts_user_id ON attempts(user_id);
CREATE INDEX idx_attempts_course_id ON attempts(course_id);
CREATE INDEX idx_attempts_user_course ON attempts(user_id, course_id);
CREATE INDEX idx_attempts_status ON attempts(status) WHERE status = 'in_progress';

-- RLS
ALTER TABLE attempts ENABLE ROW LEVEL SECURITY;

CREATE POLICY attempts_tenant_isolation ON attempts
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);

-- Триггер для автообновления updated_at
CREATE TRIGGER trg_attempts_updated_at
    BEFORE UPDATE ON attempts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE attempts IS
    'Попытки прохождения теста. attempt_number — номер попытки студента по курсу '
    '(увеличивается при каждой новой попытке). score и passed заполняются при status = completed.';

-- ============================================================
-- 3. Таблица answers
-- ============================================================

CREATE TABLE answers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    attempt_id        UUID NOT NULL REFERENCES attempts(id) ON DELETE CASCADE,
    question_id       UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,

    -- Текст ответа студента (или JSON для multiple_choice)
    answer_text       TEXT NOT NULL,

    -- Правильность ответа (заполняется при проверке)
    is_correct        BOOLEAN,

    -- Заработанные баллы
    points_earned     INTEGER NOT NULL DEFAULT 0 CHECK (points_earned >= 0),

    answered_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Объяснение правильного ответа (для неверных ответов)
    explanation       TEXT
);

-- Индексы
CREATE INDEX idx_answers_tenant_id ON answers(tenant_id);
CREATE INDEX idx_answers_attempt_id ON answers(attempt_id);
CREATE INDEX idx_answers_question_id ON answers(question_id);

-- Уникальность: один ответ на вопрос в рамках одной попытки
CREATE UNIQUE INDEX idx_answers_attempt_question ON answers(attempt_id, question_id);

-- RLS
ALTER TABLE answers ENABLE ROW LEVEL SECURITY;

CREATE POLICY answers_tenant_isolation ON answers
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);

COMMENT ON TABLE answers IS
    'Ответы студента на вопросы в рамках попытки. '
    'Один вопрос → один ответ в попытке (UNIQUE на attempt_id + question_id). '
    'is_correct и points_earned заполняются сервером при проверке.';