use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{AuthenticatedUser, CoachPlayer, DominantHand, Player, PreferredSide, User},
    error::{ErrorResponse, Result},
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreatePlayerRequest {
    pub actor_id: Uuid,
    pub user_id: Option<Uuid>,
    #[schema(example = "Lucas Aluno")]
    pub name: String,
    #[serde(default = "default_hand")]
    pub dominant_hand: DominantHand,
    #[serde(default = "default_side")]
    pub preferred_side: PreferredSide,
    #[schema(example = "3ª Categoria")]
    pub category: Option<String>,
}

fn default_hand() -> DominantHand {
    DominantHand::RightHanded
}

fn default_side() -> PreferredSide {
    PreferredSide::Left
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ActorQuery {
    pub actor_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AssignCoachRequest {
    pub actor_id: Uuid,
    pub coach_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/v1/players",
    request_body = CreatePlayerRequest,
    responses(
        (status = 201, description = "Perfil do atleta criado com sucesso", body = Player),
        (status = 400, description = "Nome vazio ou inválido", body = ErrorResponse),
        (status = 403, description = "Violação de BOLA ou permissão insuficiente", body = ErrorResponse)
    ),
    tag = "Players"
)]
pub async fn create_player(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreatePlayerRequest>,
) -> Result<(StatusCode, Json<Player>)> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let player = db
        .create_player(
            actor_id,
            payload.user_id,
            payload.name,
            payload.dominant_hand,
            payload.preferred_side,
            payload.category,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(player)))
}

#[utoipa::path(
    get,
    path = "/api/v1/players",
    params(ActorQuery),
    responses(
        (status = 200, description = "Lista de atletas associados ao treinador/ator", body = Vec<Player>),
        (status = 403, description = "Violação de BOLA", body = ErrorResponse)
    ),
    tag = "Players"
)]
pub async fn list_players(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Vec<Player>>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let players = db.list_players_for_actor(actor_id).await?;
    Ok(Json(players))
}

#[utoipa::path(
    get,
    path = "/api/v1/players/{id}",
    params(
        ("id" = Uuid, Path, description = "UUID do atleta"),
        ActorQuery
    ),
    responses(
        (status = 200, description = "Dados cadastrais do atleta", body = Player),
        (status = 403, description = "Violação de BOLA ou sem permissão de acesso ao atleta", body = ErrorResponse),
        (status = 404, description = "Atleta não encontrado", body = ErrorResponse)
    ),
    tag = "Players"
)]
pub async fn get_player(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Player>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let player = db.verify_player_read_access(actor_id, id).await?;
    Ok(Json(player))
}

#[utoipa::path(
    post,
    path = "/api/v1/players/{id}/coaches",
    params(
        ("id" = Uuid, Path, description = "UUID do atleta a receber vínculo")
    ),
    request_body = AssignCoachRequest,
    responses(
        (status = 201, description = "Vínculo treinador-atleta criado com sucesso", body = CoachPlayer),
        (status = 403, description = "Violação de BOLA ou acesso restrito", body = ErrorResponse)
    ),
    tag = "Players"
)]
pub async fn assign_coach(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(player_id): Path<Uuid>,
    Json(payload): Json<AssignCoachRequest>,
) -> Result<(StatusCode, Json<CoachPlayer>)> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let link = db
        .assign_coach_to_player(actor_id, payload.coach_id, player_id)
        .await?;

    Ok((StatusCode::CREATED, Json(link)))
}

#[utoipa::path(
    get,
    path = "/api/v1/players/{id}/coaches",
    params(
        ("id" = Uuid, Path, description = "UUID do atleta"),
        ActorQuery
    ),
    responses(
        (status = 200, description = "Lista de treinadores associados ao atleta", body = Vec<User>),
        (status = 403, description = "Violação de BOLA ou acesso restrito", body = ErrorResponse),
        (status = 404, description = "Atleta não encontrado", body = ErrorResponse)
    ),
    tag = "Players"
)]
pub async fn list_player_coaches(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(player_id): Path<Uuid>,
    Query(query): Query<ActorQuery>,
) -> Result<Json<Vec<User>>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let _player = db.verify_player_read_access(actor_id, player_id).await?;
    let coaches = db.list_coaches_for_player(player_id).await?;
    Ok(Json(coaches))
}
