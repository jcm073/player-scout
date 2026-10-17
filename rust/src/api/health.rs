use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

pub async fn health_check() -> impl IntoResponse {
    Json(HealthStatus {
        status: "ok",
        service: "player-scout-backend",
        version: env!("CARGO_PKG_VERSION"),
    })
}
