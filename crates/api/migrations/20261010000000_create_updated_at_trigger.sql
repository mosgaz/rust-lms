-- crates/api/migrations/20261010000000_create_updated_at_trigger.sql
-- Универсальная функция триггера для автообновления поля updated_at.
-- Используется во всех последующих миграциях проекта.

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_updated_at_column() IS
    'Универсальная функция триггера для автообновления поля updated_at. '
    'Применяется через CREATE TRIGGER ... BEFORE UPDATE ... EXECUTE FUNCTION update_updated_at_column().';