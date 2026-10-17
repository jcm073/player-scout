use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::players::ActorQuery,
    db::Database,
    domain::{AuthenticatedUser, MatchSession},
    error::Result,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMatchRequest {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub opponent_name: Option<String>,
    pub notes: Option<String>,
}

pub async fn create_match(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateMatchRequest>,
) -> Result<(StatusCode, Json<MatchSession>)> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let session = db
        .create_match(
            actor_id,
            payload.player_id,
            payload.opponent_name,
            payload.notes,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn get_match(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<MatchSession>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let session = db.get_match_by_id(id).await?;
    let _player = db
        .verify_player_read_access(actor_id, session.player_id)
        .await?;
    Ok(Json(session))
}
