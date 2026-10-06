-- crates/api/migrations/20261009000001_add_total_weight_to_courses.sql
--
-- Денормализация total_weight в courses.metadata для оптимизации пересчёта прогресса.
-- 
-- Проблема: Раньше total_weight вычислялся через агрегатный запрос по всем урокам курса (O(N)).
-- Для курсов с 1000+ уроков это создавало bottleneck при каждом обновлении прогресса.
--
-- Решение: Хранить total_weight в courses.metadata и обновлять через триггер при изменении уроков.
-- Это даёт O(1) доступ к total_weight вместо O(N) агрегата.
--
-- Связанные ADR: 2026.09.28-0001 (RLS), 2026.10.06-0012 (LRS)

-- ============================================================================
-- Шаг 1: Backfill существующих курсов
-- ============================================================================

-- Обновляем metadata всех существующих курсов, добавляя total_weight
UPDATE courses
SET metadata = COALESCE(metadata, '{}'::jsonb) || jsonb_build_object(
    'total_weight',
    (
        SELECT COALESCE(SUM(COALESCE((n.metadata->>'weight')::numeric, 1.0)), 0.0)
        FROM nodes n
        WHERE n.course_id = courses.id
          AND n.tenant_id = courses.tenant_id
          AND n.is_archived = FALSE
          AND n.node_type = 'lesson'
    )
);

-- ============================================================================
-- Шаг 2: Создание функции триггера для автоматического пересчёта
-- ============================================================================

CREATE OR REPLACE FUNCTION recalculate_course_total_weight()
RETURNS TRIGGER AS $$
DECLARE
    v_course_id UUID;
    v_tenant_id UUID;
    v_total_weight NUMERIC;
BEGIN
    -- Определяем course_id и tenant_id в зависимости от типа операции
    IF TG_OP = 'DELETE' THEN
        v_course_id := OLD.course_id;
        v_tenant_id := OLD.tenant_id;
    ELSIF TG_OP = 'INSERT' THEN
        v_course_id := NEW.course_id;
        v_tenant_id := NEW.tenant_id;
    ELSE -- UPDATE
        -- При UPDATE нужно обновить оба курса (старый и новый), если course_id изменился
        IF OLD.course_id != NEW.course_id THEN
            -- Обновляем старый курс
            SELECT COALESCE(SUM(COALESCE((n.metadata->>'weight')::numeric, 1.0)), 0.0)
            INTO v_total_weight
            FROM nodes n
            WHERE n.course_id = OLD.course_id
              AND n.tenant_id = OLD.tenant_id
              AND n.is_archived = FALSE
              AND n.node_type = 'lesson';
            
            UPDATE courses
            SET metadata = COALESCE(metadata, '{}'::jsonb) || jsonb_build_object('total_weight', v_total_weight)
            WHERE id = OLD.course_id
              AND tenant_id = OLD.tenant_id;
            
            -- Обновляем новый курс
            v_course_id := NEW.course_id;
            v_tenant_id := NEW.tenant_id;
        ELSE
            v_course_id := NEW.course_id;
            v_tenant_id := NEW.tenant_id;
        END IF;
    END IF;
    
    -- Вычисляем новый total_weight для курса
    SELECT COALESCE(SUM(COALESCE((n.metadata->>'weight')::numeric, 1.0)), 0.0)
    INTO v_total_weight
    FROM nodes n
    WHERE n.course_id = v_course_id
      AND n.tenant_id = v_tenant_id
      AND n.is_archived = FALSE
      AND n.node_type = 'lesson';
    
    -- Обновляем metadata курса
    UPDATE courses
    SET metadata = COALESCE(metadata, '{}'::jsonb) || jsonb_build_object('total_weight', v_total_weight)
    WHERE id = v_course_id
      AND tenant_id = v_tenant_id;
    
    -- Возвращаем запись (для INSERT/UPDATE возвращаем NEW, для DELETE возвращаем OLD)
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Шаг 3: Создание триггера на таблице nodes
-- ============================================================================

-- Удаляем старый триггер, если он существует (для идемпотентности миграции)
DROP TRIGGER IF EXISTS trg_recalculate_course_total_weight ON nodes;

-- Создаём триггер, который срабатывает после INSERT, UPDATE или DELETE
CREATE TRIGGER trg_recalculate_course_total_weight
AFTER INSERT OR UPDATE OR DELETE ON nodes
FOR EACH ROW
WHEN (
    -- Срабатываем только если изменяется урок (lesson)
    (TG_OP = 'INSERT' AND NEW.node_type = 'lesson')
    OR (TG_OP = 'DELETE' AND OLD.node_type = 'lesson')
    OR (TG_OP = 'UPDATE' AND (NEW.node_type = 'lesson' OR OLD.node_type = 'lesson'))
)
EXECUTE FUNCTION recalculate_course_total_weight();

-- ============================================================================
-- Шаг 4: Добавление комментариев для документации
-- ============================================================================

COMMENT ON COLUMN courses.metadata IS 
'Метаданные курса в формате JSONB. Содержит:
- total_weight (numeric): Суммарный вес всех неархивных уроков курса. Автоматически пересчитывается триггером trg_recalculate_course_total_weight при изменении уроков.
- completion_criteria (jsonb): Критерии завершения курса (см. CompletionCriteria в shared/models/completion.rs).
- Другие поля метаданных курса.';

COMMENT ON FUNCTION recalculate_course_total_weight() IS 
'Триггерная функция для автоматического пересчёта total_weight в courses.metadata.
Срабатывает после INSERT/UPDATE/DELETE в таблице nodes для уроков (node_type = lesson).
Учитывает только неархивные уроки (is_archived = FALSE).
Использует вес урока из nodes.metadata.weight (по умолчанию 1.0, если не задан).';

COMMENT ON TRIGGER trg_recalculate_course_total_weight ON nodes IS 
'Автоматически пересчитывает courses.metadata.total_weight при изменении уроков курса.
Оптимизация: позволяет получать total_weight за O(1) вместо O(N) агрегатного запроса.';

-- ============================================================================
-- Шаг 5: Индекс для ускорения триггера (опционально)
-- ============================================================================

-- Индекс для ускорения запроса в триггере
CREATE INDEX IF NOT EXISTS idx_nodes_course_weight_calc 
ON nodes(course_id, tenant_id, is_archived, node_type)
WHERE node_type = 'lesson' AND is_archived = FALSE;

-- ============================================================================
-- Проверка: Валидация данных после миграции
-- ============================================================================

-- Проверяем, что все курсы имеют total_weight в metadata
DO $$
DECLARE
    v_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM courses
    WHERE NOT (metadata ? 'total_weight');
    
    IF v_count > 0 THEN
        RAISE NOTICE 'Предупреждение: % курсов не имеют total_weight в metadata', v_count;
    ELSE
        RAISE NOTICE '✓ Все курсы успешно обновлены с total_weight';
    END IF;
END $$;