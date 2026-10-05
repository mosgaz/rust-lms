-- crates/api/migrations/20261003000001_init_rls_and_tenants.sql
-- Инициализация схемы БД для мультиарендной LMS-платформы.
--
-- Архитектура: Identity-First (единая личность, несколько ролей в тенантах).
-- - Таблица `identities` — глобальная сущность (email, password_hash, preferred_tenant_id).
-- - Таблица `users` — связь личности с тенантом (с RLS по tenant_id).
-- - Таблица `tenants` — арендаторы (с RLS по id).
--
-- См. ADR 2026.09.28-0001 (RLS вместо схем-per-tenant).

-- Шаг 1: Включение необходимых расширений
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Шаг 2: Создание таблицы tenants (арендаторы)
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Шаг 3: Создание глобальной таблицы identities (личности, БЕЗ RLS)
-- Хранит email, пароль и preferred_tenant_id для авто-выбора тенанта при логине.
CREATE TABLE IF NOT EXISTS identities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    preferred_tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT chk_identities_email_length CHECK (char_length(email) BETWEEN 3 AND 255),
    CONSTRAINT chk_identities_password_hash_length CHECK (char_length(password_hash) BETWEEN 10 AND 1024)
);

-- Шаг 4: Создание таблицы users (связь личности с тенантом, С RLS)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(tenant_id, identity_id)
);

-- Шаг 5: Включение Row-Level Security (RLS)
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
-- identities НЕ имеет RLS, так как это глобальная сущность

-- Шаг 6: Политики RLS для tenants
CREATE POLICY tenant_isolation_policy ON tenants
    FOR ALL
    USING (id::text = current_setting('app.current_tenant_id', true));

-- Шаг 7: Политики RLS для users
CREATE POLICY user_tenant_isolation_policy ON users
    FOR ALL
    USING (tenant_id::text = current_setting('app.current_tenant_id', true));

-- Шаг 8: Индексы для производительности RLS и поиска
CREATE INDEX IF NOT EXISTS idx_identities_email ON identities(email);
CREATE INDEX IF NOT EXISTS idx_identities_preferred_tenant_id ON identities(preferred_tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_identity_id ON users(identity_id);
CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);