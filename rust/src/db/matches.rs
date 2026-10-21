use uuid::Uuid;

use super::Database;
use crate::error::Result;

use crate::{
    db::models::{MatchPlayerRecord, MatchRecord},
    domain::{
        DominantHand, MatchPlayer, MatchSession, MatchStatus, MatchTeam, Player, PreferredSide,
    },
};

impl Database {
    pub async fn create_match(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        opponent_name: Option<String>,
        notes: Option<String>,
    ) -> Result<MatchSession> {
        let _actor = self.require_write_permission(actor_id).await?;
        let player = self.verify_player_read_access(actor_id, player_id).await?;

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

        // Registra o jogador primário na tabela associativa match_players
        let _ = toasty::create!(MatchPlayerRecord {
            match_id: session.id,
            player_id: player_id,
            team: "team_a".to_string(),
            court_side: player.preferred_side,
            is_scouted: true,
        })
        .exec(&mut db)
        .await;

        Ok(MatchSession::from(rec))
    }

    pub async fn assign_match_player(
        &self,
        match_id: Uuid,
        player_id: Uuid,
        team: MatchTeam,
        court_side: PreferredSide,
        is_scouted: bool,
    ) -> Result<MatchPlayer> {
        let mut db = self.inner.clone();
        let rec = toasty::create!(MatchPlayerRecord {
            match_id: match_id,
            player_id: player_id,
            team: team.as_str().to_string(),
            court_side: court_side,
            is_scouted: is_scouted,
        })
        .exec(&mut db)
        .await?;

        Ok(MatchPlayer::from(rec))
    }

    pub async fn list_match_players(&self, match_id: Uuid) -> Result<Vec<MatchPlayer>> {
        let mut db = self.inner.clone();
        let records =
            MatchPlayerRecord::filter(MatchPlayerRecord::fields().match_id().eq(match_id))
                .exec(&mut db)
                .await?;
        Ok(records.into_iter().map(MatchPlayer::from).collect())
    }

    pub async fn create_quick_player(
        &self,
        coach_id: Uuid,
        name: &str,
        preferred_side: PreferredSide,
    ) -> Result<Player> {
        self.create_player(
            coach_id,
            None,
            name.trim().to_string(),
            DominantHand::RightHanded,
            preferred_side,
            Some("Convidado".to_string()),
        )
        .await
    }

    pub async fn finish_match(
        &self,
        actor_id: Uuid,
        match_id: Uuid,
        score: Option<String>,
    ) -> Result<MatchSession> {
        let _actor = self.require_write_permission(actor_id).await?;
        let mut db = self.inner.clone();
        let mut rec = MatchRecord::get_by_id(&mut db, &match_id).await?;

        let mut update = rec.update().status(MatchStatus::Completed);
        if let Some(s) = score {
            update = update.score(Some(s));
        }
        update.exec(&mut db).await?;
        Ok(MatchSession::from(rec))
    }

    pub async fn get_match_by_id(&self, match_id: Uuid) -> Result<MatchSession> {
        let mut db = self.inner.clone();
        let rec = MatchRecord::get_by_id(&mut db, &match_id).await?;
        Ok(MatchSession::from(rec))
    }

    pub async fn list_matches_for_player(&self, player_id: Uuid) -> Result<Vec<MatchSession>> {
        let mut db = self.inner.clone();
        let records = MatchRecord::filter(MatchRecord::fields().player_id().eq(player_id))
            .order_by(MatchRecord::fields().id().desc())
            .exec(&mut db)
            .await?;
        Ok(records.into_iter().map(MatchSession::from).collect())
    }
}
