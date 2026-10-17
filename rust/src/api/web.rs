use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    response::Html,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    db::{Database, NewShotInput},
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, DecisionQuality, PlayerAnalyticsReport,
        ShotEvent, ShotOutcome, ShotType, SpecialTag,
    },
    error::{AppError, Result},
};

#[derive(Template)]
#[template(path = "partials/analytics_panel.html")]
pub struct AnalyticsPanelTemplate {
    pub report: PlayerAnalyticsReport,
}

#[derive(Template)]
#[template(path = "partials/compare_panel.html")]
pub struct ComparePanelTemplate {
    pub label_a: String,
    pub label_b: String,
    pub comparison: AnalyticsComparisonReport,
}

#[derive(Template)]
#[template(path = "partials/court_feed.html")]
pub struct CourtFeedTemplate {
    pub status_banner: Option<String>,
    pub report: PlayerAnalyticsReport,
    pub recent_shots: Vec<ShotEvent>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub player_name: String,
    pub analytics_html: String,
    pub compare_html: String,
}

#[derive(Template)]
#[template(path = "court.html")]
pub struct CourtTemplate {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Uuid,
    pub player_name: String,
    pub serve_shots: &'static [ShotType],
    pub net_shots: &'static [ShotType],
    pub backcourt_shots: &'static [ShotType],
    pub feed_html: String,
}

#[derive(Debug, Deserialize)]
pub struct WebDashboardQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebCourtQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct WebAnalyticsPartialQuery {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebComparePartialQuery {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_a: Option<u8>,
    pub set_b: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct CourtShotForm {
    pub actor_id: Uuid,
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
    pub actor_id: Uuid,
    pub match_id: Uuid,
}

/// Renderiza o Dashboard Analítico Web (`GET /` e `GET /web/dashboard`)
pub async fn dashboard_page(
    State(db): State<Database>,
    Query(query): Query<WebDashboardQuery>,
) -> Result<Html<String>> {
    if let (Some(actor_id), Some(player_id)) = (query.actor_id, query.player_id) {
        let player = db.verify_player_read_access(actor_id, player_id).await?;
        let filter = AnalyticsFilter {
            match_id: query.match_id,
            set_number: query.set_number,
            from_date: None,
            to_date: None,
        };
        let report = db
            .compute_player_analytics(actor_id, player_id, filter)
            .await?;

        let analytics_html = AnalyticsPanelTemplate { report }.render()?;

        let compare_html = if let Some(match_id) = query.match_id {
            let scope_a = AnalyticsFilter {
                match_id: Some(match_id),
                set_number: Some(1),
                from_date: None,
                to_date: None,
            };
            let scope_b = AnalyticsFilter {
                match_id: Some(match_id),
                set_number: Some(2),
                from_date: None,
                to_date: None,
            };
            let comparison = db
                .compare_player_analytics(actor_id, player_id, scope_a, scope_b)
                .await?;
            ComparePanelTemplate {
                label_a: "Set 1".to_string(),
                label_b: "Set 2".to_string(),
                comparison,
            }
            .render()?
        } else {
            String::new()
        };

        let html = DashboardTemplate {
            actor_id,
            player_id,
            match_id: query.match_id,
            player_name: player.name,
            analytics_html,
            compare_html,
        }
        .render()?;

        return Ok(Html(html));
    }

    // Modo inicial sem parâmetros de consulta (exibição de estrutura vazia)
    let nil_id = Uuid::nil();
    let empty_report = PlayerAnalyticsReport::from_events(nil_id, AnalyticsFilter::default(), &[]);
    let analytics_html = AnalyticsPanelTemplate {
        report: empty_report,
    }
    .render()?;

    let html = DashboardTemplate {
        actor_id: nil_id,
        player_id: nil_id,
        match_id: None,
        player_name: "Atleta Demonstração (Informe actor_id e player_id)".to_string(),
        analytics_html,
        compare_html: String::new(),
    }
    .render()?;

    Ok(Html(html))
}

/// Renderiza a Tela Touch-First de Quadra do Treinador (`GET /web/court`)
pub async fn court_page(
    State(db): State<Database>,
    Query(query): Query<WebCourtQuery>,
) -> Result<Html<String>> {
    if let (Some(actor_id), Some(player_id), Some(match_id)) =
        (query.actor_id, query.player_id, query.match_id)
    {
        let player = db.verify_player_read_access(actor_id, player_id).await?;
        let feed_html = build_court_feed_html(&db, actor_id, player_id, match_id, None).await?;

        let html = CourtTemplate {
            actor_id,
            player_id,
            match_id,
            player_name: player.name,
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
        status_banner: None,
        report: empty_report,
        recent_shots: Vec::new(),
    }
    .render()?;

    let html = CourtTemplate {
        actor_id: nil_id,
        player_id: nil_id,
        match_id: nil_id,
        player_name: "Atleta em Quadra".to_string(),
        serve_shots: ShotType::serve_shots(),
        net_shots: ShotType::net_shots(),
        backcourt_shots: ShotType::backcourt_shots(),
        feed_html,
    }
    .render()?;

    Ok(Html(html))
}

/// Fragmento HTMX: Painel Analítico Recursivo (`GET /web/partials/analytics`)
pub async fn analytics_partial(
    State(db): State<Database>,
    Query(query): Query<WebAnalyticsPartialQuery>,
) -> Result<Html<String>> {
    let filter = AnalyticsFilter {
        match_id: query.match_id,
        set_number: query.set_number,
        from_date: None,
        to_date: None,
    };
    let report = db
        .compute_player_analytics(query.actor_id, query.player_id, filter)
        .await?;

    let html = AnalyticsPanelTemplate { report }.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Modo Comparativo Lado a Lado (`GET /web/partials/compare`)
pub async fn compare_partial(
    State(db): State<Database>,
    Query(query): Query<WebComparePartialQuery>,
) -> Result<Html<String>> {
    let set_a = query.set_a.unwrap_or(1);
    let set_b = query.set_b.unwrap_or(2);

    let scope_a = AnalyticsFilter {
        match_id: query.match_id,
        set_number: Some(set_a),
        from_date: None,
        to_date: None,
    };
    let scope_b = AnalyticsFilter {
        match_id: query.match_id,
        set_number: Some(set_b),
        from_date: None,
        to_date: None,
    };

    let comparison = db
        .compare_player_analytics(query.actor_id, query.player_id, scope_a, scope_b)
        .await?;

    let html = ComparePanelTemplate {
        label_a: format!("Set {set_a}"),
        label_b: format!("Set {set_b}"),
        comparison,
    }
    .render()?;

    Ok(Html(html))
}

/// Fragmento HTMX: Registro Touch-First de Golpe em Quadra (`POST /web/partials/court/shot`)
pub async fn court_record_shot_partial(
    State(db): State<Database>,
    Form(form): Form<CourtShotForm>,
) -> Result<Html<String>> {
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
            form.actor_id,
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

    let html = build_court_feed_html(
        &db,
        form.actor_id,
        form.player_id,
        form.match_id,
        Some(banner),
    )
    .await?;

    Ok(Html(html))
}

/// Fragmento HTMX: Desfazer Última Jogada em Quadra (`POST /web/partials/court/undo`)
pub async fn court_undo_shot_partial(
    State(db): State<Database>,
    Form(form): Form<CourtUndoForm>,
) -> Result<Html<String>> {
    let undone = db.undo_last_shot(form.actor_id, form.match_id).await?;
    let banner = format!(
        "↩️ Jogada desfeita: {} (Set {})",
        undone.shot_type.label_pt(),
        undone.set_number
    );

    let html = build_court_feed_html(
        &db,
        form.actor_id,
        undone.player_id,
        form.match_id,
        Some(banner),
    )
    .await?;

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
        status_banner,
        report,
        recent_shots: shots,
    }
    .render()?;

    Ok(html)
}
