-- crates/api/migrations/20261003000001_init_rls_and_tenants.sql
-- Шаг 1: Включение необходимых расширений
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Шаг 2: Создание таблицы tenants
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Шаг 3: Создание таблицы users
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, email)
);

-- Шаг 4: Включение Row-Level Security (RLS)
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- Шаг 5: Политики RLS для tenants
-- Доступ разрешен только если id тенанта совпадает с текущей сессионной переменной
CREATE POLICY tenant_isolation_policy ON tenants
    FOR ALL
    USING (id::text = current_setting('app.current_tenant_id', true));

-- Шаг 6: Политики RLS для users
-- Доступ разрешен только если tenant_id пользователя совпадает с текущей сессионной переменной
CREATE POLICY user_tenant_isolation_policy ON users
    FOR ALL
    USING (tenant_id::text = current_setting('app.current_tenant_id', true));

-- Шаг 7: Индексы для производительности RLS и поиска
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);