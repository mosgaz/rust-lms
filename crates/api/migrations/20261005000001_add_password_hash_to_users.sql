-- crates/api/migrations/20261005000001_add_password_hash_to_users.sql
-- Добавление поля password_hash в таблицу users для поддержки аутентификации.
--
-- Хэш хранится в формате PHC string ($argon2id$...), что включает соль и параметры
-- алгоритма (см. CODING_STANDARDS.md и RFC 9106).
--
-- Примечание: для обратной совместимости с существующими записями (если они есть)
-- поле сначала добавляется как nullable, затем заполняется placeholder-значением
-- для legacy-пользователей, после чего устанавливается NOT NULL constraint.

-- Шаг 1: Добавление колонки (nullable для совместимости)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS password_hash TEXT;

-- Шаг 2: Заполнение placeholder-значением для существующих записей (если есть)
-- Значение '$argon2id$v=0$placeholder' заведомо не пройдёт верификацию,
-- но позволит установить NOT NULL constraint.
UPDATE users
SET password_hash = '$argon2id$v=0$placeholder'
WHERE password_hash IS NULL;

-- Шаг 3: Установка NOT NULL constraint
ALTER TABLE users
    ALTER COLUMN password_hash SET NOT NULL;

-- Шаг 4: Индекс для быстрого поиска по email в рамках тенанта (для логина)
CREATE INDEX IF NOT EXISTS idx_users_tenant_email
    ON users (tenant_id, email);

-- Шаг 5: Ограничение длины email для предотвращения DoS
ALTER TABLE users
    ADD CONSTRAINT chk_users_email_length
    CHECK (char_length(email) BETWEEN 3 AND 255);

-- Шаг 6: Ограничение длины password_hash (PHC-строка Argon2id обычно ~100 символов)
ALTER TABLE users
    ADD CONSTRAINT chk_users_password_hash_length
    CHECK (char_length(password_hash) BETWEEN 10 AND 1024);