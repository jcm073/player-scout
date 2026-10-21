use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthStatus {
    #[schema(example = "ok")]
    pub status: &'static str,
    #[schema(example = "player-scout-backend")]
    pub service: &'static str,
    #[schema(example = "0.1.0")]
    pub version: &'static str,
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Servidor em execução e saudável", body = HealthStatus)
    ),
    tag = "Health"
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthStatus {
        status: "ok",
        service: "player-scout-backend",
        version: env!("CARGO_PKG_VERSION"),
    })
}
