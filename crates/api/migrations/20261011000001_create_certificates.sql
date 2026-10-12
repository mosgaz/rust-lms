-- crates/api/migrations/20261011000001_create_certificates.sql
-- Миграция для Этапа 11: Сертификаты.
-- Создаёт таблицу certificates с RLS-изоляцией.
--
-- Связь с другими таблицами:
-- - tenant_id → tenants(id) ON DELETE CASCADE (RLS)
-- - user_id   → users(id)   ON DELETE CASCADE
-- - target_id — полиморфная ссылка (course_id / program_id / batch_id),
--               FK не ставим, т.к. зависит от target_type.

CREATE TABLE IF NOT EXISTS certificates (
    id                 UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id          UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    target_type        VARCHAR(32) NOT NULL
                       CHECK (target_type IN ('course', 'program', 'batch')),
    target_id          UUID NOT NULL,

    verification_hash  VARCHAR(64) NOT NULL UNIQUE,
    issued_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_certificates_verification_hash_length
        CHECK (char_length(verification_hash) BETWEEN 32 AND 128)
);

CREATE INDEX IF NOT EXISTS idx_certificates_tenant_id ON certificates(tenant_id);
CREATE INDEX IF NOT EXISTS idx_certificates_user_id ON certificates(user_id);
CREATE INDEX IF NOT EXISTS idx_certificates_target ON certificates(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_certificates_verification_hash ON certificates(verification_hash);
CREATE INDEX IF NOT EXISTS idx_certificates_issued_at ON certificates(issued_at DESC);

ALTER TABLE certificates ENABLE ROW LEVEL SECURITY;

CREATE POLICY certificates_tenant_isolation ON certificates
    USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);

CREATE TRIGGER trg_certificates_updated_at
    BEFORE UPDATE ON certificates
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE certificates IS 'Реестр выданных цифровых сертификатов';
COMMENT ON COLUMN certificates.target_type IS 'Тип сущности: course, program, batch';
COMMENT ON COLUMN certificates.target_id IS 'Идентификатор целевой сущности (course_id / program_id / batch_id)';
COMMENT ON COLUMN certificates.verification_hash IS 'SHA-256 hex-хэш для публичной верификации';