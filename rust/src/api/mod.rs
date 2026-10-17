pub mod analytics;
pub mod health;
pub mod matches;
pub mod players;
pub mod shots;
pub mod users;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::db::Database;

pub fn app_router(db: Database) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health::health_check))
        .route("/api/v1/health", get(health::health_check))
        .route(
            "/api/v1/users",
            post(users::create_user).get(users::list_users),
        )
        .route("/api/v1/users/{id}", get(users::get_user))
        .route(
            "/api/v1/players",
            post(players::create_player).get(players::list_players),
        )
        .route("/api/v1/players/{id}", get(players::get_player))
        .route(
            "/api/v1/players/{id}/coaches",
            post(players::assign_coach).get(players::list_player_coaches),
        )
        .route("/api/v1/matches", post(matches::create_match))
        .route("/api/v1/matches/{id}", get(matches::get_match))
        .route(
            "/api/v1/matches/{id}/shots",
            post(shots::record_shot).get(shots::list_shots),
        )
        .route(
            "/api/v1/matches/{id}/shots/undo",
            post(shots::undo_last_shot),
        )
        .route(
            "/api/v1/analytics/players/{id}",
            get(analytics::get_player_analytics),
        )
        .route(
            "/api/v1/analytics/compare",
            post(analytics::compare_analytics),
        )
        .with_state(db)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
