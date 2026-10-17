pub mod models;

use uuid::Uuid;

use crate::{
    db::models::{CoachPlayerRecord, MatchRecord, PlayerRecord, ShotEventRecord, UserRecord},
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, CoachPlayer, DecisionQuality, DomainError,
        DominantHand, MatchSession, Player, PlayerAnalyticsReport, PreferredSide, ShotEvent,
        ShotOutcome, ShotType, SpecialTag, User, UserRole,
    },
    error::{AppError, Result},
};

/// Repositório central de persistência sobre Toasty ORM + PostgreSQL 18
#[derive(Clone)]
pub struct Database {
    inner: toasty::Db,
}

impl Database {
    /// Inicializa o pool de conexões do Toasty ORM registrando todos os modelos e enums
    pub async fn connect(database_url: &str) -> Result<Self> {
        let db = toasty::Db::builder()
            .models(toasty::models!(
                UserRecord,
                PlayerRecord,
                CoachPlayerRecord,
                MatchRecord,
                ShotEventRecord,
            ))
            .connect(database_url)
            .await?;

        Ok(Self { inner: db })
    }

    pub fn raw(&self) -> toasty::Db {
        self.inner.clone()
    }

    // =========================================================================
    // Usuários (Admin, Coach, Player)
    // =========================================================================

    pub async fn create_user(
        &self,
        role: UserRole,
        name: String,
        email: String,
        password_hash: String,
    ) -> Result<User> {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest(
                "O nome do usuário não pode ser vazio".to_string(),
            ));
        }
        if email.trim().is_empty() {
            return Err(AppError::BadRequest(
                "O e-mail do usuário não pode ser vazio".to_string(),
            ));
        }

        let mut db = self.inner.clone();
        let rec = toasty::create!(UserRecord {
            id: Uuid::now_v7(),
            role: role,
            name: name,
            email: email,
            password_hash: password_hash,
        })
        .exec(&mut db)
        .await?;

        Ok(User::from(rec))
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User> {
        let mut db = self.inner.clone();
        let rec = UserRecord::get_by_id(&mut db, &user_id).await?;
        Ok(User::from(rec))
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let mut db = self.inner.clone();
        let records = UserRecord::all()
            .order_by(UserRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(User::from).collect())
    }

    // =========================================================================
    // Jogadores & Relacionamento N:N (Coach <-> Player) + Controle de Acesso
    // =========================================================================

    /// Verifica se o ator autenticado tem permissão de escrita (Admin ou Coach)
    pub async fn require_write_permission(&self, actor_id: Uuid) -> Result<User> {
        let actor = self.get_user_by_id(actor_id).await?;
        if !actor.role.can_write_shots() {
            return Err(AppError::Forbidden(
                "O perfil de jogador (Player) possui acesso estritamente somente leitura"
                    .to_string(),
            ));
        }
        Ok(actor)
    }

    pub async fn create_player(
        &self,
        actor_id: Uuid,
        user_id: Option<Uuid>,
        name: String,
        dominant_hand: DominantHand,
        preferred_side: PreferredSide,
        category: Option<String>,
    ) -> Result<Player> {
        let actor = self.require_write_permission(actor_id).await?;

        let primary_coach_id = if actor.role.is_coach() {
            Some(actor.id)
        } else {
            None
        };

        // Valida regras de domínio (ex: nome não vazio)
        let validated = Player::new(
            primary_coach_id,
            name,
            dominant_hand,
            preferred_side,
            category,
        )?;

        let mut db = self.inner.clone();
        let rec = toasty::create!(PlayerRecord {
            id: validated.id,
            user_id: user_id,
            coach_id: validated.coach_id,
            name: validated.name,
            dominant_hand: validated.dominant_hand,
            preferred_side: validated.preferred_side,
            category: validated.category,
        })
        .exec(&mut db)
        .await?;

        // Se criado por um Coach, registra automaticamente o vínculo N:N em coach_players
        if let Some(coach_id) = rec.coach_id {
            let _ = toasty::create!(CoachPlayerRecord {
                coach_id: coach_id,
                player_id: rec.id,
            })
            .exec(&mut db)
            .await?;
        }

        Ok(Player::from(rec))
    }

    pub async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player> {
        let mut db = self.inner.clone();
        let rec = PlayerRecord::get_by_id(&mut db, &player_id).await?;
        Ok(Player::from(rec))
    }

    /// Vincula um treinador adicional a um jogador na tabela N:N `coach_players`
    pub async fn assign_coach_to_player(
        &self,
        actor_id: Uuid,
        coach_id: Uuid,
        player_id: Uuid,
    ) -> Result<CoachPlayer> {
        let _actor = self.require_write_permission(actor_id).await?;
        let coach = self.get_user_by_id(coach_id).await?;
        if !coach.role.is_coach() && !coach.role.is_admin() {
            return Err(AppError::BadRequest(
                "O usuário atribuído como treinador deve possuir perfil Coach ou Admin".to_string(),
            ));
        }
        let _player = self.get_player_by_id(player_id).await?;

        let mut db = self.inner.clone();
        let existing = CoachPlayerRecord::filter(
            CoachPlayerRecord::fields()
                .coach_id()
                .eq(coach_id)
                .and(CoachPlayerRecord::fields().player_id().eq(player_id)),
        )
        .first()
        .exec(&mut db)
        .await?;

        if let Some(rec) = existing {
            return Ok(CoachPlayer::from(rec));
        }

        let rec = toasty::create!(CoachPlayerRecord {
            coach_id: coach_id,
            player_id: player_id,
        })
        .exec(&mut db)
        .await?;

        Ok(CoachPlayer::from(rec))
    }

    /// Lista os treinadores associados a um jogador
    pub async fn list_coaches_for_player(&self, player_id: Uuid) -> Result<Vec<User>> {
        let mut db = self.inner.clone();
        let links =
            CoachPlayerRecord::filter(CoachPlayerRecord::fields().player_id().eq(player_id))
                .exec(&mut db)
                .await?;

        if links.is_empty() {
            return Ok(Vec::new());
        }

        let coach_ids: Vec<Uuid> = links.into_iter().map(|l| l.coach_id).collect();
        let coaches = UserRecord::filter(UserRecord::fields().id().in_list(coach_ids))
            .exec(&mut db)
            .await?;

        Ok(coaches.into_iter().map(User::from).collect())
    }

    ///Verifica se um ator (`Admin`, `Coach` ou `Player`) pode visualizar os dados de um jogador
    pub async fn verify_player_read_access(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
    ) -> Result<Player> {
        let actor = self.get_user_by_id(actor_id).await?;
        let player = self.get_player_by_id(player_id).await?;

        match actor.role {
            UserRole::Admin => Ok(player),
            UserRole::Player => {
                if player.user_id == Some(actor.id) {
                    Ok(player)
                } else {
                    Err(AppError::Forbidden(
                        "Um jogador só pode visualizar o seu próprio histórico e estatísticas"
                            .to_string(),
                    ))
                }
            }
            UserRole::Coach => {
                if player.coach_id == Some(actor.id) {
                    return Ok(player);
                }
                let mut db = self.inner.clone();
                let link = CoachPlayerRecord::filter(
                    CoachPlayerRecord::fields()
                        .coach_id()
                        .eq(actor.id)
                        .and(CoachPlayerRecord::fields().player_id().eq(player_id)),
                )
                .first()
                .exec(&mut db)
                .await?;

                if link.is_some() {
                    Ok(player)
                } else {
                    Err(AppError::Forbidden(
                        "Treinador não possui vínculo ativo com este jogador".to_string(),
                    ))
                }
            }
        }
    }

    /// Lista os jogadores visíveis para o usuário autenticado conforme seu perfil:
    /// - `Admin`: todos os jogadores do sistema
    /// - `Coach`: todos os jogadores vinculados em `coach_players`
    /// - `Player`: estritamente o seu próprio registro de atleta
    pub async fn list_players_for_actor(&self, actor_id: Uuid) -> Result<Vec<Player>> {
        let actor = self.get_user_by_id(actor_id).await?;
        let mut db = self.inner.clone();

        match actor.role {
            UserRole::Admin => {
                let records = PlayerRecord::all()
                    .order_by(PlayerRecord::fields().id().asc())
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
            UserRole::Player => {
                let records = PlayerRecord::filter(PlayerRecord::fields().user_id().eq(actor.id))
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
            UserRole::Coach => {
                let links =
                    CoachPlayerRecord::filter(CoachPlayerRecord::fields().coach_id().eq(actor.id))
                        .exec(&mut db)
                        .await?;

                if links.is_empty() {
                    return Ok(Vec::new());
                }

                let player_ids: Vec<Uuid> = links.into_iter().map(|l| l.player_id).collect();
                let records = PlayerRecord::filter(PlayerRecord::fields().id().in_list(player_ids))
                    .order_by(PlayerRecord::fields().id().asc())
                    .exec(&mut db)
                    .await?;
                Ok(records.into_iter().map(Player::from).collect())
            }
        }
    }

    // =========================================================================
    // Partidas / Sessões (Matches)
    // =========================================================================

    pub async fn create_match(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        opponent_name: Option<String>,
        notes: Option<String>,
    ) -> Result<MatchSession> {
        let _actor = self.require_write_permission(actor_id).await?;
        let _player = self.verify_player_read_access(actor_id, player_id).await?;

        let session = MatchSession::new(actor_id, player_id, opponent_name);
        let mut db = self.inner.clone();
        let rec = toasty::create!(MatchRecord {
            id: session.id,
            coach_id: session.coach_id,
            player_id: session.player_id,
            tournament_id: session.tournament_id,
            opponent_name: session.opponent_name,
            status: session.status,
            score: session.score,
            notes: notes,
        })
        .exec(&mut db)
        .await?;

        Ok(MatchSession::from(rec))
    }

    pub async fn get_match_by_id(&self, match_id: Uuid) -> Result<MatchSession> {
        let mut db = self.inner.clone();
        let rec = MatchRecord::get_by_id(&mut db, &match_id).await?;
        Ok(MatchSession::from(rec))
    }
}

/// Parâmetros de entrada para o registro de uma jogada em quadra
#[derive(Debug, Clone)]
pub struct NewShotInput {
    pub player_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: Option<DecisionQuality>,
    pub special_tag: Option<SpecialTag>,
}

impl Database {
    // =========================================================================
    // Jogadas em Quadra (Shot Events: Ingestão Rápida & Undo)
    // =========================================================================

    pub async fn record_shot(
        &self,
        actor_id: Uuid,
        match_id: Uuid,
        input: NewShotInput,
    ) -> Result<ShotEvent> {
        let _actor = self.require_write_permission(actor_id).await?;
        let match_session = self.get_match_by_id(match_id).await?;

        if match_session.player_id != input.player_id {
            return Err(AppError::BadRequest(
                "O jogador informado não pertence a esta partida".to_string(),
            ));
        }

        // Valida regras de negócio do Padel (Winner -> Hit, UnforcedError -> Miss, Set 1..=5)
        let validated = ShotEvent::new(
            match_id,
            input.player_id,
            input.set_number,
            input.shot_type,
            input.outcome,
            input.decision,
            input.special_tag,
        )?;

        let mut db = self.inner.clone();
        let rec = toasty::create!(ShotEventRecord {
            id: validated.id,
            match_id: validated.match_id,
            player_id: validated.player_id,
            set_number: validated.set_number,
            shot_type: validated.shot_type,
            outcome: validated.outcome,
            decision: validated.decision,
            special_tag: validated.special_tag,
        })
        .exec(&mut db)
        .await?;

        Ok(ShotEvent::from(rec))
    }

    pub async fn list_shots_by_match(
        &self,
        actor_id: Uuid,
        match_id: Uuid,
    ) -> Result<Vec<ShotEvent>> {
        let match_session = self.get_match_by_id(match_id).await?;
        let _player = self
            .verify_player_read_access(actor_id, match_session.player_id)
            .await?;

        let mut db = self.inner.clone();
        let records = ShotEventRecord::filter(ShotEventRecord::fields().match_id().eq(match_id))
            .order_by(ShotEventRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(ShotEvent::from).collect())
    }

    /// Desfaz (remove) a última jogada registrada em uma partida usando ordenação decrescente por UUIDv7
    pub async fn undo_last_shot(&self, actor_id: Uuid, match_id: Uuid) -> Result<ShotEvent> {
        let _actor = self.require_write_permission(actor_id).await?;
        let match_session = self.get_match_by_id(match_id).await?;
        let _player = self
            .verify_player_read_access(actor_id, match_session.player_id)
            .await?;

        let mut db = self.inner.clone();
        let last_shot = ShotEventRecord::filter(ShotEventRecord::fields().match_id().eq(match_id))
            .order_by(ShotEventRecord::fields().id().desc())
            .first()
            .exec(&mut db)
            .await?;

        let Some(rec) = last_shot else {
            return Err(AppError::NotFound(
                "Nenhuma jogada registrada nesta partida para desfazer".to_string(),
            ));
        };

        let undone = ShotEvent::from(rec.clone());
        rec.delete().exec(&mut db).await?;

        Ok(undone)
    }

    // =========================================================================
    // Motor Analítico Recursivo & Modo Comparativo (Fase 3)
    // =========================================================================

    pub async fn query_player_shots(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
    ) -> Result<Vec<ShotEvent>> {
        let _player = self.verify_player_read_access(actor_id, player_id).await?;
        let mut db = self.inner.clone();

        let records = ShotEventRecord::filter(ShotEventRecord::fields().player_id().eq(player_id))
            .order_by(ShotEventRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(ShotEvent::from).collect())
    }

    pub async fn compute_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        filter: AnalyticsFilter,
    ) -> Result<PlayerAnalyticsReport> {
        if let Some(set_num) = filter.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        Ok(PlayerAnalyticsReport::from_events(
            player_id, filter, &events,
        ))
    }

    pub async fn compare_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        scope_a: AnalyticsFilter,
        scope_b: AnalyticsFilter,
    ) -> Result<AnalyticsComparisonReport> {
        if let Some(set_num) = scope_a.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }
        if let Some(set_num) = scope_b.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        let report_a = PlayerAnalyticsReport::from_events(player_id, scope_a, &events);
        let report_b = PlayerAnalyticsReport::from_events(player_id, scope_b, &events);

        Ok(AnalyticsComparisonReport::compare(report_a, report_b))
    }
}
