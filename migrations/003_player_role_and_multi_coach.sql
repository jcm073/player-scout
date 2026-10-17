-- Player Scout: Migração 003 - Perfil Player e Relacionamento N:N Treinador-Jogador
-- PostgreSQL 18+

-- 1. Adiciona perfil 'player' ao enum de perfis de usuário (se ainda não adicionado)
ALTER TYPE user_role_enum ADD VALUE IF NOT EXISTS 'player';

-- 2. Adiciona user_id opcional na tabela de atletas (permitindo login read-only do próprio atleta)
ALTER TABLE players ADD COLUMN IF NOT EXISTS user_id UUID UNIQUE REFERENCES users(id) ON DELETE SET NULL;

-- 3. Torna coach_id opcional em players para suportar o relacionamento flexível N:N
ALTER TABLE players ALTER COLUMN coach_id DROP NOT NULL;

-- 4. Criação da tabela associativa Coach <-> Player (N:N)
-- Permite que um jogador treine com múltiplos coaches (ex: técnico e físico)
CREATE TABLE IF NOT EXISTS coach_players (
    coach_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (coach_id, player_id)
);

CREATE INDEX IF NOT EXISTS idx_coach_players_coach_id ON coach_players(coach_id);
CREATE INDEX IF NOT EXISTS idx_coach_players_player_id ON coach_players(player_id);

-- 5. Migra dados existentes de players.coach_id para a tabela associativa coach_players
INSERT INTO coach_players (coach_id, player_id)
SELECT coach_id, id FROM players
WHERE coach_id IS NOT NULL
ON CONFLICT DO NOTHING;
