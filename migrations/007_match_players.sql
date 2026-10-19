-- Migração 007: Tabela associativa match_players para suporte a até 4 atletas por partida
-- Permite persistir Dupla A, Dupla B, posições de quadra (Revés/Drive) e flag de scout ativo.

CREATE TABLE IF NOT EXISTS match_players (
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    team VARCHAR(10) NOT NULL CHECK (team IN ('team_a', 'team_b')),
    court_side preferred_side_enum NOT NULL,
    is_scouted BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (match_id, player_id)
);

CREATE INDEX IF NOT EXISTS idx_match_players_player_id ON match_players(player_id);
CREATE INDEX IF NOT EXISTS idx_match_players_match_id ON match_players(match_id);
