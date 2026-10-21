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
    error::{ErrorResponse, Result},
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateMatchRequest {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    #[schema(example = "Dupla Rival")]
    pub opponent_name: Option<String>,
    #[schema(example = "Final do torneio regional")]
    pub notes: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/matches",
    request_body = CreateMatchRequest,
    responses(
        (status = 201, description = "Partida criada com sucesso", body = MatchSession),
        (status = 400, description = "Dados da partida inválidos", body = ErrorResponse),
        (status = 403, description = "Violação de BOLA ou permissão insuficiente", body = ErrorResponse)
    ),
    tag = "Matches"
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/matches/{id}",
    params(
        ("id" = Uuid, Path, description = "UUID da partida"),
        ActorQuery
    ),
    responses(
        (status = 200, description = "Dados detalhados da sessão de partida", body = MatchSession),
        (status = 403, description = "Violação de BOLA ou sem acesso ao jogador da partida", body = ErrorResponse),
        (status = 404, description = "Partida não encontrada", body = ErrorResponse)
    ),
    tag = "Matches"
)]
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
