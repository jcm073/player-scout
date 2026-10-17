-- Player Scout: Migração 004 - Cadastro Completo (Player/Coach), CPF/Email Únicos, Argon2i e Contas OAuth2
-- PostgreSQL 18+

-- 1. Adiciona colunas obrigatórias do cadastro completo na tabela users
-- (Mantendo compatibilidade com registros existentes via valores default ou nullable em nível físico e validação estrita na camada de domínio/API)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS first_name VARCHAR(100),
    ADD COLUMN IF NOT EXISTS last_name VARCHAR(150),
    ADD COLUMN IF NOT EXISTS age SMALLINT CHECK (age IS NULL OR (age >= 5 AND age <= 120)),
    ADD COLUMN IF NOT EXISTS cpf VARCHAR(11) UNIQUE,
    ADD COLUMN IF NOT EXISTS whatsapp_phone VARCHAR(20),
    ADD COLUMN IF NOT EXISTS cep VARCHAR(8),
    ADD COLUMN IF NOT EXISTS street VARCHAR(255),
    ADD COLUMN IF NOT EXISTS street_number VARCHAR(30),
    ADD COLUMN IF NOT EXISTS complement VARCHAR(10) CHECK (complement IS NULL OR char_length(complement) <= 10),
    ADD COLUMN IF NOT EXISTS neighborhood VARCHAR(120),
    ADD COLUMN IF NOT EXISTS city VARCHAR(120),
    ADD COLUMN IF NOT EXISTS state VARCHAR(2),
    ADD COLUMN IF NOT EXISTS preferred_side preferred_side_enum NOT NULL DEFAULT 'left';

-- 2. Índices para busca rápida por chaves de identificação (CPF e Email)
CREATE INDEX IF NOT EXISTS idx_users_cpf ON users(cpf);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- 3. Enum de Provedores OAuth2 suportados (Gmail/Google, Microsoft Office e Instagram)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'oauth_provider_enum') THEN
        CREATE TYPE oauth_provider_enum AS ENUM ('google', 'microsoft', 'instagram');
    END IF;
END$$;

-- 4. Tabela de vínculos de contas sociais OAuth2 (user_oauth_accounts)
CREATE TABLE IF NOT EXISTS user_oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider oauth_provider_enum NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    provider_email VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (provider, provider_user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_oauth_user_id ON user_oauth_accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_provider_email ON user_oauth_accounts(provider, provider_email);
