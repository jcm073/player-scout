use askama::Template;
use axum::{
    Extension,
    extract::{Query, State},
    response::Html,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, AuthenticatedUser, PlayerAnalyticsReport,
    },
    error::Result,
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
