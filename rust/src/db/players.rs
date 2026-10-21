use uuid::Uuid;

use super::Database;
use crate::error::{AppError, Result};

use crate::{
    db::models::{CoachPlayerRecord, PlayerRecord, UserRecord},
    domain::{CoachPlayer, DominantHand, Player, PreferredSide, User, UserRole},
};

impl Database {
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
}
