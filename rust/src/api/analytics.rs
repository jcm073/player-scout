use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Database,
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, AuthenticatedUser, PlayerAnalyticsReport,
    },
    error::Result,
};

#[derive(Debug, Deserialize)]
pub struct PlayerAnalyticsQuery {
    pub actor_id: Uuid,
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompareAnalyticsRequest {
    pub actor_id: Uuid,
    pub player_id: Uuid,
    pub scope_a: AnalyticsFilter,
    pub scope_b: AnalyticsFilter,
}

/// Endpoint de Agregação Recursiva:
/// `GET /api/v1/analytics/players/{id}` com filtros opcionais (`match_id`, `set_number`, `from_date`, `to_date`)
pub async fn get_player_analytics(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(player_id): Path<Uuid>,
    Query(query): Query<PlayerAnalyticsQuery>,
) -> Result<Json<PlayerAnalyticsReport>> {
    let actor_id = auth_user.resolve_actor_id(Some(query.actor_id))?;
    let filter = AnalyticsFilter {
        match_id: query.match_id,
        set_number: query.set_number,
        from_date: query.from_date,
        to_date: query.to_date,
    };

    let report = db
        .compute_player_analytics(actor_id, player_id, filter)
        .await?;

    Ok(Json(report))
}

/// Endpoint de Modo Comparativo Lado a Lado:
/// `POST /api/v1/analytics/compare` comparando Escopo A vs Escopo B com cálculo de deltas (B - A)
pub async fn compare_analytics(
    State(db): State<Database>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CompareAnalyticsRequest>,
) -> Result<Json<AnalyticsComparisonReport>> {
    let actor_id = auth_user.resolve_actor_id(Some(payload.actor_id))?;
    let comparison = db
        .compare_player_analytics(
            actor_id,
            payload.player_id,
            payload.scope_a,
            payload.scope_b,
        )
        .await?;

    Ok(Json(comparison))
}
