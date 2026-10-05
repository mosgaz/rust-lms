-- crates/api/migrations/20261006000001_create_content_hierarchy.sql
-- Иерархия контента: таблицы courses и nodes.
--
-- Используется паттерн Adjacency List + ltree для эффективных запросов поддеревьев.
-- - `parent_id` — источник правды для структуры (целостность через FK + ON DELETE CASCADE).
-- - `path` (ltree) — денормализованное представление для быстрых запросов поддеревьев.
-- - `node_type` — тип узла (program/course/chapter/topic/lesson).

-- Шаг 1: Включение расширения ltree (если ещё не включено)
CREATE EXTENSION IF NOT EXISTS ltree;

-- Шаг 2: Создание таблицы courses (если её нет в init миграции)
CREATE TABLE IF NOT EXISTS courses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    title_i18n JSONB,
    description TEXT,
    description_i18n JSONB,
    version INTEGER NOT NULL DEFAULT 1,
    certification_rules JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE courses ENABLE ROW LEVEL SECURITY;

CREATE POLICY courses_tenant_isolation_policy ON courses
    FOR ALL USING (tenant_id::text = current_setting('app.current_tenant_id', true));

CREATE INDEX IF NOT EXISTS idx_courses_tenant_id ON courses(tenant_id);

-- Шаг 3: Создание типа enum для node_type (если не существует)
DO $$ BEGIN
    CREATE TYPE node_type AS ENUM ('program', 'course', 'chapter', 'topic', 'lesson');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Шаг 4: Создание таблицы nodes
CREATE TABLE IF NOT EXISTS nodes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES nodes(id) ON DELETE CASCADE,
    path ltree NOT NULL,
    node_type node_type NOT NULL,
    course_id UUID REFERENCES courses(id) ON DELETE CASCADE,
    title VARCHAR(512) NOT NULL,
    title_i18n JSONB,
    description TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_nodes_title_length CHECK (char_length(title) BETWEEN 1 AND 512)
);

-- Шаг 5: Включение Row-Level Security для nodes
ALTER TABLE nodes ENABLE ROW LEVEL SECURITY;

-- Шаг 6: Политика RLS (tenant_id изоляция)
CREATE POLICY nodes_tenant_isolation_policy ON nodes
    FOR ALL
    USING (tenant_id::text = current_setting('app.current_tenant_id', true));

-- Шаг 7: Индексы для производительности
CREATE INDEX IF NOT EXISTS idx_nodes_tenant_id ON nodes(tenant_id);
CREATE INDEX IF NOT EXISTS idx_nodes_parent_id ON nodes(parent_id);
CREATE INDEX IF NOT EXISTS idx_nodes_course_id ON nodes(course_id);

-- GiST-индекс для ltree-операторов (@>, <@, ~)
CREATE INDEX IF NOT EXISTS idx_nodes_path ON nodes USING gist(path);

-- GIN-индекс для JSONB metadata
CREATE INDEX IF NOT EXISTS idx_nodes_metadata ON nodes USING gin(metadata jsonb_path_ops);

-- Индекс для сортировки детей внутри родителя
CREATE INDEX IF NOT EXISTS idx_nodes_sort_order ON nodes(parent_id, sort_order);

-- Шаг 8: Комментарии для документации
COMMENT ON TABLE nodes IS 'Единая таблица для иерархии контента: program/course/chapter/topic/lesson';
COMMENT ON COLUMN nodes.path IS 'ltree-путь для быстрых запросов поддеревьев (обновляется в приложении)';
COMMENT ON COLUMN nodes.node_type IS 'Тип узла: program, course, chapter, topic, lesson';
COMMENT ON COLUMN nodes.course_id IS 'Связь с курсом (для chapter/topic/lesson); NULL для program и изолированных lesson';
COMMENT ON COLUMN nodes.metadata IS 'Расширяемое содержимое узла (video, document, quiz, assignment и т.д.)';