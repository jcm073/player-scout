use askama::Template;
use axum::{
    Extension, Form,
    extract::{Query, State},
    response::Html,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    db::{Database, NewShotInput},
    domain::{
        AnalyticsFilter, AuthenticatedUser, DecisionQuality, PlayerAnalyticsReport, ShotEvent,
        ShotOutcome, ShotType, SpecialTag,
    },
    error::{AppError, Result},
};

#[derive(Debug, Clone)]
pub struct CourtAthleteView {
    pub player_id: Uuid,
    pub name: String,
    pub team: String,
    pub team_label: String,
    pub court_side: String,
    pub is_scouted: bool,
    pub color_hex: String,
    pub in_percentage: u32,
    pub hits: usize,
    pub total: usize,
}

#[derive(Template)]
#[template(path = "court.html")]
pub struct CourtTemplate {
    pub actor_id: Uuid,
    pub match_id: Uuid,
    pub player_id: Uuid,
    pub player_name: String,
    pub match_score: String,
    pub current_set: u8,
    pub athletes: Vec<CourtAthleteView>,
    pub serve_shots: &'static [ShotType],
    pub net_shots: &'static [ShotType],
    pub backcourt_shots: &'static [ShotType],
    pub feed_html: String,
}

#[derive(Template)]
#[template(path = "partials/court_feed.html")]
pub struct CourtFeedTemplate {
    pub player_id: Uuid,
    pub status_banner: Option<String>,
    pub report: PlayerAnalyticsReport,
    pub recent_shots: Vec<ShotEvent>,
}

#[derive(Debug, Deserialize)]
pub struct WebCourtQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CourtShotForm {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: Option<DecisionQuality>,
    pub special_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CourtUndoForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
}

/// Renderiza a Tela Touch-First de Quadra do Treinador (`GET /web/court`)
pub async fn court_page(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebCourtQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;

    if let Some(match_id) = query.match_id {
        let match_session = db.get_match_by_id(match_id).await?;
        let match_players = db.list_match_players(match_id).await.unwrap_or_default();

        let mut athlete_views = Vec::new();
        let colors = ["#06b6d4", "#eab308", "#8b5cf6", "#10b981"]; // Cyan, Amarelo, Roxo, Verde

        for (idx, mp) in match_players.iter().enumerate() {
            if let Ok(p) = db.get_player_by_id(mp.player_id).await {
                let filter = AnalyticsFilter {
                    match_id: Some(match_id),
                    set_number: None,
                    from_date: None,
                    to_date: None,
                };
                let report = db
                    .compute_player_analytics(actor_id, p.id, filter)
                    .await
                    .unwrap_or_else(|_| {
                        PlayerAnalyticsReport::from_events(p.id, AnalyticsFilter::default(), &[])
                    });

                athlete_views.push(CourtAthleteView {
                    player_id: p.id,
                    name: p.name,
                    team: mp.team.as_str().to_string(),
                    team_label: mp.team.label_pt().to_string(),
                    court_side: mp.court_side.label_pt().to_string(),
                    is_scouted: mp.is_scouted,
                    color_hex: colors[idx % colors.len()].to_string(),
                    in_percentage: report.hit_rate_pct as u32,
                    hits: report.hits,
                    total: report.total_shots,
                });
            }
        }

        // Se match_players estiver vazio (retrocompatibilidade), carrega o player_id da partida
        if athlete_views.is_empty()
            && let Ok(p) = db.get_player_by_id(match_session.player_id).await
        {
            athlete_views.push(CourtAthleteView {
                player_id: p.id,
                name: p.name,
                team: "team_a".to_string(),
                team_label: "Dupla A".to_string(),
                court_side: p.preferred_side.label_pt().to_string(),
                is_scouted: true,
                color_hex: "#06b6d4".to_string(),
                in_percentage: 0,
                hits: 0,
                total: 0,
            });
        }

        let active_player_id = query.player_id.unwrap_or_else(|| {
            athlete_views
                .first()
                .map(|a| a.player_id)
                .unwrap_or(match_session.player_id)
        });

        let active_player_name = athlete_views
            .iter()
            .find(|a| a.player_id == active_player_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Atleta em Quadra".to_string());

        let feed_html =
            build_court_feed_html(&db, actor_id, active_player_id, match_id, None).await?;

        let html = CourtTemplate {
            actor_id,
            match_id,
            player_id: active_player_id,
            player_name: active_player_name,
            match_score: match_session.score.unwrap_or_else(|| "0 - 0".to_string()),
            current_set: 1,
            athletes: athlete_views,
            serve_shots: ShotType::serve_shots(),
            net_shots: ShotType::net_shots(),
            backcourt_shots: ShotType::backcourt_shots(),
            feed_html,
        }
        .render()?;

        return Ok(Html(html));
    }

    let nil_id = Uuid::nil();
    let empty_report = PlayerAnalyticsReport::from_events(nil_id, AnalyticsFilter::default(), &[]);
    let feed_html = CourtFeedTemplate {
        player_id: nil_id,
        status_banner: None,
        report: empty_report,
        recent_shots: Vec::new(),
    }
    .render()?;

    let html = CourtTemplate {
        actor_id,
        match_id: nil_id,
        player_id: nil_id,
        player_name: "Atleta em Quadra".to_string(),
        match_score: "0 - 0".to_string(),
        current_set: 1,
        athletes: Vec::new(),
        serve_shots: ShotType::serve_shots(),
        net_shots: ShotType::net_shots(),
        backcourt_shots: ShotType::backcourt_shots(),
        feed_html,
    }
    .render()?;

    Ok(Html(html))
}

/// Fragmento HTMX: Registro Touch-First de Golpe em Quadra (`POST /web/partials/court/shot`)
pub async fn court_record_shot_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<CourtShotForm>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(form.actor_id)?;
    let parsed_special_tag = match form.special_tag.as_deref().map(str::trim) {
        None | Some("") => None,
        Some("winner") => Some(SpecialTag::Winner),
        Some("unforced_error") => Some(SpecialTag::UnforcedError),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "Marcador especial inválido: {other}"
            )));
        }
    };

    // Regra de ergonomia Touch-First:
    // - Se o treinador marcou "Erro Não Forçado (ENF)", garante automaticamente `Miss`
    // - Se o treinador marcou "Winner", garante automaticamente `Hit`
    let effective_outcome = match parsed_special_tag {
        Some(SpecialTag::UnforcedError) => ShotOutcome::Miss,
        Some(SpecialTag::Winner) => ShotOutcome::Hit,
        None => form.outcome,
    };

    let recorded = db
        .record_shot(
            actor_id,
            form.match_id,
            NewShotInput {
                player_id: form.player_id,
                set_number: form.set_number,
                shot_type: form.shot_type,
                outcome: effective_outcome,
                decision: form.decision,
                special_tag: parsed_special_tag,
            },
        )
        .await?;

    let outcome_label = match recorded.outcome {
        ShotOutcome::Hit => "ACERTOU",
        ShotOutcome::Miss => "ERROU",
    };
    let banner = format!(
        "Golpe registrado: {} ({}) — Set {}",
        recorded.shot_type.label_pt(),
        outcome_label,
        recorded.set_number
    );

    let html =
        build_court_feed_html(&db, actor_id, form.player_id, form.match_id, Some(banner)).await?;

    Ok(Html(html))
}

/// Fragmento HTMX: Desfazer Última Jogada em Quadra (`POST /web/partials/court/undo`)
pub async fn court_undo_shot_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<CourtUndoForm>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(form.actor_id)?;
    let undone = db.undo_last_shot(actor_id, form.match_id).await?;
    let banner = format!(
        "↩️ Jogada desfeita: {} (Set {})",
        undone.shot_type.label_pt(),
        undone.set_number
    );

    let html =
        build_court_feed_html(&db, actor_id, undone.player_id, form.match_id, Some(banner)).await?;

    Ok(Html(html))
}

async fn build_court_feed_html(
    db: &Database,
    actor_id: Uuid,
    player_id: Uuid,
    match_id: Uuid,
    status_banner: Option<String>,
) -> Result<String> {
    let mut shots = db.list_shots_by_match(actor_id, match_id).await?;
    let filter = AnalyticsFilter {
        match_id: Some(match_id),
        set_number: None,
        from_date: None,
        to_date: None,
    };
    let report = PlayerAnalyticsReport::from_events(player_id, filter, &shots);

    shots.reverse();
    shots.truncate(5);

    let html = CourtFeedTemplate {
        player_id,
        status_banner,
        report,
        recent_shots: shots,
    }
    .render()?;

    Ok(html)
}
