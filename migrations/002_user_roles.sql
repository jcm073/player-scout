-- Player Scout: Migração 002 - Introdução de Perfis de Usuário (Admin e Coach)
-- PostgreSQL 18+

-- 1. Cria enum de perfis de usuário
CREATE TYPE user_role_enum AS ENUM ('admin', 'coach');

-- 2. Transforma a tabela coaches em users com role
ALTER TABLE coaches RENAME TO users;
ALTER TABLE users ADD COLUMN role user_role_enum NOT NULL DEFAULT 'coach';

-- 3. Índice para consultas rápidas por perfil de usuário
CREATE INDEX idx_users_role ON users(role);
