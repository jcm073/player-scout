use askama::Template;
use axum::{
    Extension, Form,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::auth::{
        LoginRequest, append_security_headers, build_clear_oauth_state_cookie,
        build_clear_session_cookie, build_oauth_state_cookie, build_session_cookie,
        extract_oauth_state_from_headers, extract_token_from_headers,
    },
    db::{Database, NewShotInput},
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, AuthenticatedUser, DecisionQuality,
        MatchSession, MatchTeam, OAuthProvider, Player, PlayerAnalyticsReport, PreferredSide,
        RegistrationInput, ShotEvent, ShotOutcome, ShotType, SpecialTag,
    },
    error::{AppError, Result},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {}

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
    pub player_id: Uuid,
    pub status_banner: Option<String>,
    pub report: PlayerAnalyticsReport,
    pub recent_shots: Vec<ShotEvent>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub actor_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub user_role: String,
    pub user_preferred_side: String,
    pub user_preferred_side_key: String,
    pub user_whatsapp: String,
    pub user_initials: String,
    pub can_write_shots: bool,
    pub just_finished: bool,
    pub player_id: Uuid,
    pub player_name: String,
    pub player_side: String,
    pub has_valid_player: bool,
    pub match_id: Option<Uuid>,
    pub matches: Vec<MatchSession>,
    pub available_players: Vec<Player>,
    pub analytics_html: String,
    pub compare_html: String,
}

impl DashboardTemplate {
    pub fn is_selected_player(&self, id: &Uuid) -> bool {
        self.player_id == *id
    }

    pub fn is_selected_match(&self, id: &Uuid) -> bool {
        self.match_id.as_ref() == Some(id)
    }

    pub fn is_career_scope(&self) -> bool {
        self.match_id.is_none()
    }
}

pub fn calculate_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.is_empty() {
        "PS".to_string()
    } else if parts.len() == 1 {
        parts[0].chars().take(2).collect::<String>().to_uppercase()
    } else {
        format!(
            "{}{}",
            parts[0].chars().next().unwrap_or('P'),
            parts[parts.len() - 1].chars().next().unwrap_or('S')
        )
        .to_uppercase()
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileForm {
    pub name: String,
    pub preferred_side: PreferredSide,
    pub whatsapp_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

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
#[template(path = "match_setup.html")]
pub struct MatchSetupTemplate {
    pub actor_id: Uuid,
    pub user_name: String,
    pub available_players: Vec<Player>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebDashboardQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
    pub just_finished: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WebCourtQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct WebAnalyticsPartialQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebComparePartialQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_a: Option<u8>,
    pub set_b: Option<u8>,
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

/// Renderiza o Dashboard Analítico Web (`GET /` e `GET /web/dashboard`)
pub async fn dashboard_page(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebDashboardQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
    let user_name = auth_user.user.name.clone();
    let user_email = auth_user.user.email.clone();
    let user_role = auth_user.user.role.as_str().to_string();
    let user_preferred_side = auth_user.user.preferred_side.label_pt().to_string();
    let user_preferred_side_key = auth_user.user.preferred_side.as_str().to_string();
    let user_whatsapp = auth_user.user.whatsapp_phone.clone().unwrap_or_default();
    let user_initials = calculate_initials(&user_name);
    let can_write_shots = auth_user.user.role.can_write_shots();

    // 1. Resolve os atletas disponíveis para este usuário autenticado
    let available_players = db
        .list_players_for_actor(actor_id)
        .await
        .unwrap_or_default();

    // 2. Determina o jogador ativo:
    //    - Se query.player_id foi informado (e não é nil), valida o acesso de leitura
    //    - Caso contrário, seleciona o primeiro atleta da lista de disponíveis (se houver)
    let resolved_player = if let Some(pid) = query.player_id.filter(|id| !id.is_nil()) {
        db.verify_player_read_access(actor_id, pid).await.ok()
    } else {
        available_players.first().cloned()
    };

    if let Some(player) = resolved_player {
        let player_id = player.id;
        let player_name = player.name.clone();
        let player_side = player.preferred_side.label_pt().to_string();

        let matches = db
            .list_matches_for_player(player_id)
            .await
            .unwrap_or_default();

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
            user_name,
            user_email,
            user_role,
            user_preferred_side,
            user_preferred_side_key,
            user_whatsapp,
            user_initials,
            can_write_shots,
            just_finished: query.just_finished.unwrap_or(false),
            player_id,
            player_name,
            player_side,
            has_valid_player: true,
            match_id: query.match_id,
            matches,
            available_players,
            analytics_html,
            compare_html,
        }
        .render()?;

        return Ok(Html(html));
    }

    // Modo sem atleta vinculado (ex: treinador recém-cadastrado sem atletas vinculados ainda)
    let nil_id = Uuid::nil();
    let empty_report = PlayerAnalyticsReport::from_events(nil_id, AnalyticsFilter::default(), &[]);
    let analytics_html = AnalyticsPanelTemplate {
        report: empty_report,
    }
    .render()?;

    let html = DashboardTemplate {
        actor_id,
        user_name,
        user_email,
        user_role,
        user_preferred_side,
        user_preferred_side_key,
        user_whatsapp,
        user_initials,
        can_write_shots,
        just_finished: query.just_finished.unwrap_or(false),
        player_id: nil_id,
        player_name: "Nenhum atleta vinculado".to_string(),
        player_side: "Drive / Revés".to_string(),
        has_valid_player: false,
        match_id: None,
        matches: Vec::new(),
        available_players: Vec::new(),
        analytics_html,
        compare_html: String::new(),
    }
    .render()?;

    Ok(Html(html))
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

/// Fragmento HTMX: Painel Analítico Recursivo (`GET /web/partials/analytics`)
pub async fn analytics_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebAnalyticsPartialQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
    let filter = AnalyticsFilter {
        match_id: query.match_id,
        set_number: query.set_number,
        from_date: None,
        to_date: None,
    };
    let report = db
        .compute_player_analytics(actor_id, query.player_id, filter)
        .await?;

    let html = AnalyticsPanelTemplate { report }.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Modo Comparativo Lado a Lado (`GET /web/partials/compare`)
pub async fn compare_partial(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<WebComparePartialQuery>,
) -> Result<Html<String>> {
    let actor_id = auth_user.resolve_actor_id(query.actor_id)?;
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
        .compare_player_analytics(actor_id, query.player_id, scope_a, scope_b)
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

#[derive(Debug, Deserialize)]
pub struct RematchForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
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

#[derive(Debug, Deserialize)]
pub struct QuickPlayerForm {
    pub actor_id: Option<Uuid>,
    pub name: String,
    pub preferred_side: String,
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

#[derive(Debug, Deserialize)]
pub struct FinishMatchForm {
    pub actor_id: Option<Uuid>,
    pub match_id: Uuid,
    pub score: Option<String>,
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

/// Renderiza a Tela de Login (`GET /web/login`)
pub async fn login_page(
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Html<String>> {
    let error = match query.get("reason").map(String::as_str) {
        Some("session_required") => {
            Some("Sessão obrigatória ou expirada. Faça login para continuar.".to_string())
        }
        _ => None,
    };
    let html = LoginTemplate { error }.render()?;
    Ok(Html(html))
}

/// Processa Login por Formulário Web Padrão (`POST /web/login`)
/// - Redireciona imediatamente para o Dashboard Analítico (HTTP 303 See Other)
/// - Emite o cookie HttpOnly `ps_session` com duração de 12 horas
/// - Em caso de credenciais inválidas, recarrega o formulário exibindo a mensagem de erro
pub async fn web_login(State(db): State<Database>, Form(form): Form<LoginRequest>) -> Response {
    match db.authenticate_by_email(&form.email, &form.password).await {
        Ok((user, player_profile, session_token, _session)) => {
            let dashboard_link = if let Some(player) = &player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
            if let Ok(loc) = HeaderValue::from_str(&dashboard_link) {
                headers.insert(header::LOCATION, loc.clone());
                headers.insert(HeaderName::from_static("hx-redirect"), loc);
            }
            append_security_headers(&mut headers);

            (StatusCode::SEE_OTHER, headers).into_response()
        }
        Err(err) => {
            let html = LoginTemplate {
                error: Some(format!("Falha no login: {err}")),
            }
            .render()
            .unwrap_or_default();

            let mut headers = HeaderMap::new();
            append_security_headers(&mut headers);
            (StatusCode::UNAUTHORIZED, headers, Html(html)).into_response()
        }
    }
}

/// Renderiza a Tela de Cadastro Completo (`GET /web/register`)
pub async fn register_page() -> Result<Html<String>> {
    let html = RegisterTemplate {}.render()?;
    Ok(Html(html))
}

/// Fragmento HTMX: Processa Login por E-mail (`POST /web/partials/auth/login`)
/// - Emite o cookie HttpOnly `ps_session` com duração de 12 horas
/// - Redireciona instantaneamente o cliente sem exibir banner intermediário
pub async fn web_login_partial(
    State(db): State<Database>,
    Form(form): Form<LoginRequest>,
) -> Result<Response> {
    match db.authenticate_by_email(&form.email, &form.password).await {
        Ok((user, player_profile, session_token, _session)) => {
            let dashboard_link = if let Some(player) = &player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
            if let Ok(redirect_val) = HeaderValue::from_str(&dashboard_link) {
                headers.insert(header::LOCATION, redirect_val.clone());
                headers.insert(HeaderName::from_static("hx-redirect"), redirect_val);
            }
            append_security_headers(&mut headers);

            // Redirecionamento automático sem exibir mensagem intermediária verde
            let html = format!(
                r#"<script>window.location.replace("{}");</script>"#,
                dashboard_link
            );

            Ok((headers, Html(html)).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-4 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Falha no login: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}

/// Fragmento HTMX: Processa Cadastro Completo de Player ou Coach (`POST /web/partials/auth/register`)
pub async fn web_register_partial(
    State(db): State<Database>,
    Form(form): Form<RegistrationInput>,
) -> Result<Response> {
    match db.register_user_with_profile(form).await {
        Ok((user, player_profile)) => {
            let profile_note = if let Some(player) = &player_profile {
                format!(
                    "Perfil de Atleta vinculado com sucesso (ID: <code>{}</code>, Posição: <strong>{}</strong>).",
                    player.id,
                    player.preferred_side.label_pt()
                )
            } else {
                format!(
                    "Perfil de Treinador ativo (Posição de referência: <strong>{}</strong>).",
                    user.preferred_side.label_pt()
                )
            };

            let html = format!(
                r#"<div class="p-4 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs space-y-2">
                    <div class="font-extrabold text-sm text-emerald-300">✅ Cadastro concluído com sucesso!</div>
                    <div>Usuário: <strong>{}</strong> · CPF: <code>{}</code> · E-mail: <code>{}</code></div>
                    <div>{}</div>
                    <div class="text-[11px] text-emerald-400/90">Faça login com seu e-mail e senha para acessar o sistema.</div>
                    <div class="pt-1">
                      <a href="/web/login" class="inline-block px-3.5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-bold transition">Ir para o Login →</a>
                    </div>
                </div>"#,
                user.name,
                user.cpf.as_deref().unwrap_or(""),
                user.email,
                profile_note
            );

            let mut headers = HeaderMap::new();
            append_security_headers(&mut headers);

            Ok((headers, Html(html)).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-4 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Erro ao realizar cadastro: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}

/// Endpoint Web de Logoff (`POST /web/auth/logout`)
/// - Exclui fisicamente o token da sessão do banco de dados
/// - Remove o cookie `ps_session` (`Max-Age=0`) e redireciona para `/web/login`
pub async fn web_logout(State(db): State<Database>, headers: HeaderMap) -> Response {
    if let Some(token) = extract_token_from_headers(&headers) {
        let _ = db.revoke_session_by_token(&token).await;
    }

    let is_htmx = headers
        .get("HX-Request")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));

    let mut resp = if is_htmx {
        let mut r = Html(String::new()).into_response();
        if let Ok(hv) = HeaderValue::from_str("/web/login") {
            r.headers_mut().insert("HX-Redirect", hv);
        }
        r
    } else {
        Redirect::to("/web/login").into_response()
    };

    if let Ok(cookie_val) = HeaderValue::from_str(&build_clear_session_cookie()) {
        resp.headers_mut().insert(header::SET_COOKIE, cookie_val);
    }
    append_security_headers(resp.headers_mut());
    resp
}

#[derive(Debug, Deserialize)]
pub struct WebOAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub email: Option<String>,
    pub provider_user_id: Option<String>,
}

/// Inicia o fluxo de Login Social OAuth2 pelo navegador (`GET /web/auth/oauth/{provider}/authorize`)
/// - Gera `state` Anti-CSRF e armazena no cookie `HttpOnly` `ps_oauth_state` (10 min)
/// - Redireciona (`302 Found`) para a página de autorização oficial do provedor (`Google`, `Microsoft`, `Instagram`)
pub async fn web_oauth_authorize(Path(provider): Path<OAuthProvider>) -> Response {
    let env_prefix = match provider {
        OAuthProvider::Google => "OAUTH_GOOGLE",
        OAuthProvider::Microsoft => "OAUTH_MICROSOFT",
        OAuthProvider::Instagram => "OAUTH_INSTAGRAM",
    };

    let client_id_var = format!("{env_prefix}_CLIENT_ID");
    let redirect_var = format!("{env_prefix}_REDIRECT_URI");

    let client_id = std::env::var(&client_id_var)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("pending_{}_client_id", provider.as_str()));
    let redirect_uri = std::env::var(&redirect_var).unwrap_or_else(|_| {
        format!(
            "http://localhost:3000/web/auth/oauth/{}/callback",
            provider.as_str()
        )
    });

    let state = Uuid::now_v7().to_string();
    let authorize_url = provider.authorize_url(&client_id, &redirect_uri, &state);
    let state_cookie = build_oauth_state_cookie(&state);

    let mut resp = Redirect::to(&authorize_url).into_response();
    if let Ok(hv) = HeaderValue::from_str(&state_cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    append_security_headers(resp.headers_mut());
    resp
}

/// Callback Web do fluxo OAuth2 (`GET /web/auth/oauth/{provider}/callback`):
/// - Valida o parâmetro `state` contra o cookie `ps_oauth_state` (Defesa Anti-CSRF)
/// - Caso o e-mail já possua cadastro ativo, emite a sessão única de 12 horas (`ps_session`) e redireciona para `/web/dashboard`
/// - Caso o e-mail ainda não possua cadastro completo (CPF/WhatsApp/endereço), redireciona para `/web/register?email=...&oauth_provider=...`
pub async fn web_oauth_callback(
    State(db): State<Database>,
    headers: HeaderMap,
    Path(provider): Path<OAuthProvider>,
    Query(query): Query<WebOAuthCallbackQuery>,
) -> Result<Response> {
    if let Some(cookie_state) = extract_oauth_state_from_headers(&headers) {
        let provided = query.state.as_deref().unwrap_or("").trim();
        if provided.is_empty() || provided != cookie_state {
            return Err(AppError::Forbidden(
                "Falha na validação Anti-CSRF do callback OAuth2 (state inválido ou divergente)"
                    .to_string(),
            ));
        }
    } else if query.state.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Err(AppError::Forbidden(
            "Callback OAuth2 rejeitado: parâmetro Anti-CSRF state ausente".to_string(),
        ));
    }

    let provider_email = query
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest(
                "E-mail do provedor OAuth2 ausente no callback de homologação".to_string(),
            )
        })?
        .to_lowercase();

    let provider_user_id = query
        .provider_user_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let code_suffix = query.code.as_deref().unwrap_or("oauth");
            format!("{}_{}", provider.as_str(), code_suffix)
        });

    match db
        .authenticate_with_oauth(provider, provider_user_id, provider_email.clone())
        .await
    {
        Ok((user, player_profile, _oauth_acc, session_token, _session)) => {
            let dashboard_url = if let Some(player) = player_profile {
                format!(
                    "/web/dashboard?actor_id={}&player_id={}",
                    user.id, player.id
                )
            } else {
                format!("/web/dashboard?actor_id={}&player_id={}", user.id, user.id)
            };

            let mut resp = Redirect::to(&dashboard_url).into_response();
            if let Ok(hv) = HeaderValue::from_str(&build_session_cookie(&session_token)) {
                resp.headers_mut().insert(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(AppError::NotFound(_)) => {
            let register_url = format!(
                "/web/register?email={}&oauth_provider={}",
                provider_email,
                provider.as_str()
            );
            let mut resp = Redirect::to(&register_url).into_response();
            if let Ok(hv) = HeaderValue::from_str(&build_clear_oauth_state_cookie()) {
                resp.headers_mut().insert(header::SET_COOKIE, hv);
            }
            append_security_headers(resp.headers_mut());
            Ok(resp)
        }
        Err(err) => Err(err),
    }
}

/// Fragmento HTMX: Atualiza Dados Cadastrais / Preferências do Usuário (`POST /web/preferences/profile`)
pub async fn web_update_profile(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<UpdateProfileForm>,
) -> Result<Html<String>> {
    let clean_whatsapp = form.whatsapp_phone.filter(|s| !s.trim().is_empty());
    match db
        .update_user_profile(
            auth_user.user.id,
            &form.name,
            form.preferred_side,
            clean_whatsapp,
        )
        .await
    {
        Ok(user) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs flex items-center justify-between">
                    <span>✅ Preferências salvas: <strong>{}</strong> (Posição: <strong>{}</strong>). Recarregando...</span>
                    <button type="button" onclick="window.location.reload()" class="px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-[11px] transition">Recarregar</button>
                </div>
                <script>setTimeout(() => window.location.reload(), 1200);</script>"#,
                user.name,
                user.preferred_side.label_pt()
            );
            Ok(Html(html))
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Erro ao atualizar perfil: {}
                </div>"#,
                err
            );
            Ok(Html(html))
        }
    }
}

/// Fragmento HTMX: Atualiza Senha do Usuário com Rotação de Sessão (`POST /web/preferences/password`)
pub async fn web_update_password(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Form(form): Form<UpdatePasswordForm>,
) -> Result<Response> {
    if form.new_password != form.confirm_password {
        let html = r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
            ❌ A confirmação da nova senha não confere.
        </div>"#;
        return Ok(Html(html.to_string()).into_response());
    }

    match db
        .change_user_password(
            auth_user.user.id,
            &form.current_password,
            &form.new_password,
        )
        .await
    {
        Ok((_user, new_token, _session)) => {
            let html = r#"<div class="p-3.5 rounded-xl bg-emerald-950/80 border border-emerald-700 text-emerald-200 text-xs font-semibold">
                ✅ Senha alterada com sucesso! Sessão renovada com segurança.
            </div>"#;

            let mut headers = HeaderMap::new();
            if let Ok(cookie_val) = HeaderValue::from_str(&build_session_cookie(&new_token)) {
                headers.insert(header::SET_COOKIE, cookie_val);
            }
            append_security_headers(&mut headers);

            Ok((headers, Html(html.to_string())).into_response())
        }
        Err(err) => {
            let html = format!(
                r#"<div class="p-3.5 rounded-xl bg-rose-950/80 border border-rose-700 text-rose-200 text-xs font-semibold">
                    ❌ Falha ao alterar senha: {}
                </div>"#,
                err
            );
            Ok(Html(html).into_response())
        }
    }
}
