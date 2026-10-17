use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{CoachPlayer, DominantHand, Player, PreferredSide, User},
    error::Result,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePlayerRequest {
    pub actor_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    #[serde(default = "default_hand")]
    pub dominant_hand: DominantHand,
    #[serde(default = "default_side")]
    pub preferred_side: PreferredSide,
    pub category: Option<String>,
}

fn default_hand() -> DominantHand {
    DominantHand::RightHanded
}

fn default_side() -> PreferredSide {
    PreferredSide::Left
}

#[derive(Debug, Deserialize)]
pub struct ActorQuery {
    pub actor_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssignCoachRequest {
    pub actor_id: Uuid,
    pub coach_id: Uuid,
}

pub async fn create_player(
    State(db): State<Database>,
    Json(payload): Json<CreatePlayerRequest>,
) -> Result<(StatusCode, Json<Player>)> {
    let player = db
        .create_player(
            payload.actor_id,
            payload.user_id,
            payload.name,
            payload.dominant_hand,
            payload.preferred_side,
            payload.category,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(player)))
}

pub async fn list_players(
    State(db): State<Database>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Vec<Player>>> {
    let players = db.list_players_for_actor(query.actor_id).await?;
    Ok(Json(players))
}

pub async fn get_player(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Player>> {
    let player = db.verify_player_read_access(query.actor_id, id).await?;
    Ok(Json(player))
}

pub async fn assign_coach(
    State(db): State<Database>,
    Path(player_id): Path<Uuid>,
    Json(payload): Json<AssignCoachRequest>,
) -> Result<(StatusCode, Json<CoachPlayer>)> {
    let link = db
        .assign_coach_to_player(payload.actor_id, payload.coach_id, player_id)
        .await?;

    Ok((StatusCode::CREATED, Json(link)))
}

pub async fn list_player_coaches(
    State(db): State<Database>,
    Path(player_id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Vec<User>>> {
    let _player = db
        .verify_player_read_access(query.actor_id, player_id)
        .await?;
    let coaches = db.list_coaches_for_player(player_id).await?;
    Ok(Json(coaches))
}
