use uuid::Uuid;

use super::Database;
use crate::error::{AppError, Result};

use crate::{
    db::models::{MatchPlayerRecord, ShotEventRecord},
    domain::{DecisionQuality, ShotEvent, ShotOutcome, ShotType, SpecialTag},
};

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

        // Validação multi-atleta: verifica se o jogador é o titular da partida OU se está escalado em match_players
        let is_valid_player = if match_session.player_id == input.player_id {
            true
        } else {
            let mut db = self.inner.clone();
            let link = MatchPlayerRecord::filter(
                MatchPlayerRecord::fields()
                    .match_id()
                    .eq(match_id)
                    .and(MatchPlayerRecord::fields().player_id().eq(input.player_id)),
            )
            .first()
            .exec(&mut db)
            .await?;
            link.is_some()
        };

        if !is_valid_player {
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
}
