use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::players::ActorQuery, db::Database, domain::MatchSession, error::Result};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMatchRequest {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub opponent_name: Option<String>,
    pub notes: Option<String>,
}

pub async fn create_match(
    State(db): State<Database>,
    Json(payload): Json<CreateMatchRequest>,
) -> Result<(StatusCode, Json<MatchSession>)> {
    let session = db
        .create_match(
            payload.actor_id,
            payload.player_id,
            payload.opponent_name,
            payload.notes,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn get_match(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<MatchSession>> {
    let session = db.get_match_by_id(id).await?;
    let _player = db
        .verify_player_read_access(query.actor_id, session.player_id)
        .await?;
    Ok(Json(session))
}
