use askama::Template;
use axum::{
    Extension, Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use super::dashboard::WebDashboardQuery;
use crate::{
    db::Database,
    domain::{AuthenticatedUser, MatchTeam, Player, PreferredSide},
    error::Result,
};

#[derive(Template)]
#[template(path = "match_setup.html")]
pub struct MatchSetupTemplate {
    pub actor_id: Uuid,
    pub user_name: String,
    pub available_players: Vec<Player>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMatchWebForm {
    pub actor_id: Option<Uuid>,
    pub p1_id: Option<Uuid>,
    pub p1_scouted: Option<String>,
    pub p2_id: Option<Uuid>,
    pub p2_scouted: Option<String>,
    pub p3_id: Option<Uuid>,
    pub p3_scouted: Option<String>,
    pub p4_id: Option<Uuid>,
    pub p4_scouted: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RematchForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct QuickPlayerForm {
    pub actor_id: Option<Uuid>,
    pub name: String,
    pub preferred_side: String,
}

#[derive(Debug, Deserialize)]
pub struct FinishMatchForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
    pub score: Option<String>,
}

/// Renderiza a Página de Seleção & Setup de Partida (`GET /web/matches/setup`)
pub async fn match_setup_page(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebDashboardQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
    let available_players = db
        .list_players_for_actor(actor_id)
        .await
        .unwrap_or_default();

    let html = MatchSetupTemplate {
        actor_id,
        user_name: auth_user.user.name,
        available_players,
        error: None,
    }
    .render()?;

    Ok(Html(html))
}

/// Cria nova partida multi-atleta e persiste escalação em match_players (`POST /web/matches/new`)
pub async fn create_match_web(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<CreateMatchWebForm>,
) -> Response {
    let actor_id = match auth_user.resolve_actor_id(form.actor_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let primary_player_id = match form.p1_id {
        Some(id) if !id.is_nil() => id,
        _ => {
            let available = db
                .list_players_for_actor(actor_id)
                .await
                .unwrap_or_default();
            match available.first() {
                Some(p) => p.id,
                None => {
                    return (StatusCode::BAD_REQUEST, "Nenhum atleta selecionado").into_response();
                }
            }
        }
    };

    let match_session = match db
        .create_match(actor_id, primary_player_id, None, None)
        .await
    {
        Ok(m) => m,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // Associa P2 na Dupla A (Drive)
    if let Some(p2) = form
        .p2_id
        .filter(|id| !id.is_nil() && *id != primary_player_id)
    {
        let is_scouted = form.p2_scouted.as_deref() == Some("true");
        let _ = db
            .assign_match_player(
                match_session.id,
                p2,
                MatchTeam::TeamA,
                PreferredSide::Right,
                is_scouted,
            )
            .await;
    }

    // Associa P3 na Dupla B (Revés)
    if let Some(p3) = form.p3_id.filter(|id| !id.is_nil()) {
        let is_scouted = form.p3_scouted.as_deref() == Some("true");
        let _ = db
            .assign_match_player(
                match_session.id,
                p3,
                MatchTeam::TeamB,
                PreferredSide::Left,
                is_scouted,
            )
            .await;
    }

    // Associa P4 na Dupla B (Drive)
    if let Some(p4) = form.p4_id.filter(|id| !id.is_nil()) {
        let is_scouted = form.p4_scouted.as_deref() == Some("true");
        let _ = db
            .assign_match_player(
                match_session.id,
                p4,
                MatchTeam::TeamB,
                PreferredSide::Right,
                is_scouted,
            )
            .await;
    }

    Redirect::to(&format!(
        "/web/court?actor_id={}&match_id={}",
        actor_id, match_session.id
    ))
    .into_response()
}

/// Cria nova partida com os mesmos atletas da anterior (`POST /web/matches/rematch`)
pub async fn rematch_web(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<RematchForm>,
) -> Response {
    let actor_id = match auth_user.resolve_actor_id(form.actor_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let previous_match = match db.get_match_by_id(form.match_id).await {
        Ok(m) => m,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };

    let players = db
        .list_match_players(form.match_id)
        .await
        .unwrap_or_default();
    let primary_player_id = players
        .first()
        .map(|p| p.player_id)
        .unwrap_or(previous_match.player_id);

    let new_match = match db
        .create_match(
            actor_id,
            primary_player_id,
            previous_match.opponent_name,
            None,
        )
        .await
    {
        Ok(m) => m,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // Associa os demais jogadores com os mesmos papéis
    for mp in players.iter().skip(1) {
        let _ = db
            .assign_match_player(
                new_match.id,
                mp.player_id,
                mp.team,
                mp.court_side,
                mp.is_scouted,
            )
            .await;
    }

    Redirect::to(&format!(
        "/web/court?actor_id={}&match_id={}",
        actor_id, new_match.id
    ))
    .into_response()
}

/// Criação rápida de atleta sparring / convidado (`POST /web/matches/quick-player`)
pub async fn create_quick_player_web(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<QuickPlayerForm>,
) -> Response {
    let actor_id = match auth_user.resolve_actor_id(form.actor_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let side = match form.preferred_side.as_str() {
        "right" => PreferredSide::Right,
        _ => PreferredSide::Left,
    };

    let _ = db.create_quick_player(actor_id, &form.name, side).await;

    Redirect::to(&format!("/web/matches/setup?actor_id={}", actor_id)).into_response()
}

/// Finaliza partida de quadra e redireciona para resumo pós-partida (`POST /web/court/match/finish`)
pub async fn finish_match_web(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<FinishMatchForm>,
) -> Response {
    let actor_id = match auth_user.resolve_actor_id(form.actor_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let score = form.score.filter(|s| !s.trim().is_empty());
    let _ = db.finish_match(actor_id, form.match_id, score).await;

    Redirect::to(&format!(
        "/?actor_id={}&match_id={}&just_finished=true",
        actor_id, form.match_id
    ))
    .into_response()
}
