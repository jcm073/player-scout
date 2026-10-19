-- Player Scout: Migração 006 - Desativação Lógica de Usuários e Treinadores (Soft Delete)
-- PostgreSQL 18+

-- 1. Adiciona a coluna is_active com default true na tabela users
-- (Garante que usuários existentes permaneçam ativos sem interrupção de serviço)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true;

-- 2. Índice para consultas rápidas de usuários ativos/inativos e autenticação
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);
