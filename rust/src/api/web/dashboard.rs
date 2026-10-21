use askama::Template;
use axum::{
    Extension,
    extract::{Query, State},
    response::Html,
};
use serde::Deserialize;
use uuid::Uuid;

use super::analytics::{AnalyticsPanelTemplate, ComparePanelTemplate};
use crate::{
    db::Database,
    domain::{AnalyticsFilter, AuthenticatedUser, MatchSession, Player, PlayerAnalyticsReport},
    error::Result,
};

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
pub struct WebDashboardQuery {
    pub actor_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
    pub just_finished: Option<bool>,
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
