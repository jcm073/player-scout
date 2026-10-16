-- Player Scout: Migração Inicial 001 - Esquema Base do Padel
-- PostgreSQL 18+

-- Habilita extensão para geração de UUID
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Enums de Domínio
CREATE TYPE dominant_hand_enum AS ENUM ('right_handed', 'left_handed');
CREATE TYPE preferred_side_enum AS ENUM ('left', 'right', 'both');
CREATE TYPE match_status_enum AS ENUM ('in_progress', 'completed', 'canceled');

-- Catálogo Oficial de 21 Jogadas do Padel
CREATE TYPE shot_type_enum AS ENUM (
    -- Início de Ponto
    'saque',
    'recepcao',
    -- Rede & Ataque
    'voleio_direita',
    'voleio_esquerda',
    'bandeja',
    'vibora',
    'smash_potencia',
    'smash_x3',
    'smash_x4',
    'rulo',
    'deixadinha',
    'bloqueio',
    -- Fundo & Vidros
    'batida_direita',
    'batida_esquerda',
    'globo_direita',
    'globo_esquerda',
    'chiquita',
    'saida_parede_direita',
    'saida_parede_esquerda',
    'bajada_parede',
    'contra_parede'
);

-- Resultado da Jogada (Obrigatório)
CREATE TYPE shot_outcome_enum AS ENUM ('hit', 'miss');

-- Qualidade da Decisão Tática (Padrão: good)
CREATE TYPE decision_quality_enum AS ENUM ('good', 'bad');

-- Marcadores Especiais (Opcional)
CREATE TYPE special_tag_enum AS ENUM ('winner', 'unforced_error');

-- 1. Tabela de Treinadores / Coaches
CREATE TABLE coaches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 2. Tabela de Jogadores / Atletas
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id UUID NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    dominant_hand dominant_hand_enum NOT NULL DEFAULT 'right_handed',
    preferred_side preferred_side_enum NOT NULL DEFAULT 'left',
    category VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. Tabela de Torneios / Eventos
CREATE TABLE tournaments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id UUID NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    location VARCHAR(255),
    start_date DATE,
    end_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 4. Tabela de Partidas / Sessões de Treino
CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coach_id UUID NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    tournament_id UUID REFERENCES tournaments(id) ON DELETE SET NULL,
    opponent_name VARCHAR(255),
    match_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status match_status_enum NOT NULL DEFAULT 'in_progress',
    score VARCHAR(50),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 5. Tabela de Eventos de Jogada (Shot Events)
CREATE TABLE shot_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    set_number SMALLINT NOT NULL CHECK (set_number >= 1 AND set_number <= 5),
    shot_type shot_type_enum NOT NULL,
    outcome shot_outcome_enum NOT NULL,
    decision decision_quality_enum NOT NULL DEFAULT 'good',
    special_tag special_tag_enum,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Regra de Consistência 1: Winner é sempre 'hit' (acertou)
    CONSTRAINT chk_winner_must_be_hit 
        CHECK (special_tag IS DISTINCT FROM 'winner' OR outcome = 'hit'),

    -- Regra de Consistência 2: Erro Não Forçado é sempre 'miss' (errou)
    CONSTRAINT chk_unforced_error_must_be_miss 
        CHECK (special_tag IS DISTINCT FROM 'unforced_error' OR outcome = 'miss')
);

-- Índices de Alta Performance para Agregações Recursivas e Filtros
CREATE INDEX idx_players_coach_id ON players(coach_id);
CREATE INDEX idx_matches_player_date ON matches(player_id, match_date);
CREATE INDEX idx_matches_tournament ON matches(tournament_id);

-- Índices cruciais para o motor analítico e dashboard
CREATE INDEX idx_shot_events_match_set ON shot_events(match_id, set_number, created_at);
CREATE INDEX idx_shot_events_player_created ON shot_events(player_id, created_at);
CREATE INDEX idx_shot_events_type_outcome ON shot_events(shot_type, outcome);
CREATE INDEX idx_shot_events_decision ON shot_events(decision);
