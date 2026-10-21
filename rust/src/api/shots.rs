use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::players::ActorQuery,
    db::{Database, NewShotInput},
    domain::{AuthenticatedUser, DecisionQuality, ShotEvent, ShotOutcome, ShotType, SpecialTag},
    error::{ErrorResponse, Result},
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct RecordShotRequest {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    #[schema(example = 1)]
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: Option<DecisionQuality>,
    pub special_tag: Option<SpecialTag>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UndoShotRequest {
    pub actor_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/v1/matches/{id}/shots",
    params(
        ("id" = Uuid, Path, description = "UUID da partida")
    ),
    request_body = RecordShotRequest,
    responses(
        (status = 201, description = "Golpe registrado com sucesso", body = ShotEvent),
        (status = 400, description = "Violação de regras de golpe (ex: Winner com Miss ou ENF com Hit)", body = ErrorResponse),
        (status = 403, description = "Violação de BOLA ou perfil sem permissão de escrita", body = ErrorResponse)
    ),
    tag = "Shots"
)]
pub async fn record_shot(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(match_id): Path<Uuid>,
    Json(payload): Json<RecordShotRequest>,
) -> Result<(StatusCode, Json<ShotEvent>)> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let shot = db
        .record_shot(
            actor_id,
            match_id,
            NewShotInput {
                player_id: payload.player_id,
                set_number: payload.set_number,
                shot_type: payload.shot_type,
                outcome: payload.outcome,
                decision: payload.decision,
                special_tag: payload.special_tag,
            },
        )
        .await?;

    Ok((StatusCode::CREATED, Json(shot)))
}

#[utoipa::path(
    get,
    path = "/api/v1/matches/{id}/shots",
    params(
        ("id" = Uuid, Path, description = "UUID da partida"),
        ActorQuery
    ),
    responses(
        (status = 200, description = "Lista ordenada de golpes da partida", body = Vec<ShotEvent>),
        (status = 403, description = "Violação de BOLA ou sem acesso à partida", body = ErrorResponse)
    ),
    tag = "Shots"
)]
pub async fn list_shots(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(match_id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Vec<ShotEvent>>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let shots = db.list_shots_by_match(actor_id, match_id).await?;
    Ok(Json(shots))
}

#[utoipa::path(
    post,
    path = "/api/v1/matches/{id}/shots/undo",
    params(
        ("id" = Uuid, Path, description = "UUID da partida")
    ),
    request_body = UndoShotRequest,
    responses(
        (status = 200, description = "Último golpe desfeito e removido com sucesso", body = ShotEvent),
        (status = 400, description = "Nenhum golpe registrado para desfazer", body = ErrorResponse),
        (status = 403, description = "Violação de BOLA", body = ErrorResponse)
    ),
    tag = "Shots"
)]
pub async fn undo_last_shot(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(match_id): Path<Uuid>,
    Json(payload): Json<UndoShotRequest>,
) -> Result<Json<ShotEvent>> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let undone = db.undo_last_shot(actor_id, match_id).await?;
    Ok(Json(undone))
}
