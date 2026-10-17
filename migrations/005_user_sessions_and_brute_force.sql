-- =============================================================================
-- Player Scout - Migração 005: Controle de Sessão Única (12h) & Proteção contra Força Bruta
-- =============================================================================

-- 1. Controle de Força Bruta na tabela de usuários (`users`):
--    - `failed_login_attempts`: contador de falhas consecutivas de senha
--    - `locked_until_epoch`: timestamp Unix (segundos UTC) até quando o login está bloqueado
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS failed_login_attempts INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS locked_until_epoch BIGINT;

-- 2. Tabela de Sessões Autenticadas (`user_sessions`):
--    - `UNIQUE (user_id)` garante no nível do banco de dados que NUNCA existirão
--      múltiplas sessões simultâneas para o mesmo usuário.
--    - `token_hash` armazena exclusivamente o hash Argon2i PHC do segredo do token
--      (reutilizando a mesma biblioteca `argon2` das senhas).
--    - `expires_at_epoch` define a expiração estrita de 12 horas (43.200 segundos).
--    - Sessões expiradas ou encerradas via logoff sofrem `DELETE` físico imediato.
CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    created_at_epoch BIGINT NOT NULL,
    expires_at_epoch BIGINT NOT NULL,
    CONSTRAINT chk_user_sessions_ttl CHECK (expires_at_epoch > created_at_epoch)
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at
    ON user_sessions (expires_at_epoch);
