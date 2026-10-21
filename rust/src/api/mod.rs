pub mod analytics;
pub mod auth;
pub mod health;
pub mod matches;
pub mod players;
pub mod shots;
pub mod users;
pub mod web;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::db::Database;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::register,
        auth::login,
        auth::logout,
        auth::session_status,
        auth::change_password,
        auth::lookup_user,
        auth::oauth_authorize,
        auth::oauth_login,
        auth::oauth_callback,
        users::create_user,
        users::get_user,
        users::list_users,
        users::deactivate_user,
        users::reactivate_user,
        players::create_player,
        players::list_players,
        players::get_player,
        players::assign_coach,
        players::list_player_coaches,
        matches::create_match,
        matches::get_match,
        shots::record_shot,
        shots::list_shots,
        shots::undo_last_shot,
        analytics::get_player_analytics,
        analytics::compare_analytics,
    ),
    components(
        schemas(
            crate::error::ErrorResponse,
            health::HealthStatus,
            auth::AuthResponse,
            auth::LogoutResponse,
            auth::LoginRequest,
            auth::ChangePasswordRequest,
            auth::ChangePasswordResponse,
            auth::OAuthAuthorizeResponse,
            auth::OAuthCallbackRequest,
            auth::OAuthLoginRequest,
            auth::OAuthLoginResponse,
            users::CreateUserRequest,
            players::CreatePlayerRequest,
            players::AssignCoachRequest,
            matches::CreateMatchRequest,
            shots::RecordShotRequest,
            shots::UndoShotRequest,
            analytics::CompareAnalyticsRequest,
            crate::domain::User,
            crate::domain::UserRole,
            crate::domain::UserSession,
            crate::domain::RegistrationInput,
            crate::domain::OAuthProvider,
            crate::domain::UserOAuthAccount,
            crate::domain::AuthenticatedUser,
            crate::domain::Player,
            crate::domain::DominantHand,
            crate::domain::PreferredSide,
            crate::domain::CoachPlayer,
            crate::domain::MatchSession,
            crate::domain::MatchStatus,
            crate::domain::MatchTeam,
            crate::domain::MatchPlayer,
            crate::domain::ShotEvent,
            crate::domain::ShotType,
            crate::domain::ShotOutcome,
            crate::domain::DecisionQuality,
            crate::domain::SpecialTag,
            crate::domain::AnalyticsFilter,
            crate::domain::DecisionExecutionMatrix,
            crate::domain::ZoneStat,
            crate::domain::ZoneMetrics,
            crate::domain::PadelRadarMetrics,
            crate::domain::ShotTypeBreakdown,
            crate::domain::PlayerAnalyticsReport,
            crate::domain::ShotTypeComparisonDelta,
            crate::domain::ComparisonDeltas,
            crate::domain::AnalyticsComparisonReport,
        )
    ),
    tags(
        (name = "Health", description = "Monitoramento e status de integridade do serviço"),
        (name = "Auth", description = "Autenticação, sessões de 12h (Argon2i), cadastro e segurança"),
        (name = "OAuth2", description = "Login Social e vinculação de contas (Google, Microsoft, Instagram)"),
        (name = "Users", description = "Gestão de usuários, perfis de acesso e desativação/reativação"),
        (name = "Players", description = "Gestão de atletas, preferências de quadra e vínculos com treinadores"),
        (name = "Matches", description = "Setup de partidas de padel e acompanhamento de jogos"),
        (name = "Shots", description = "Ingestão e scouting de golpes em tempo real na quadra (21 golpes)"),
        (name = "Analytics", description = "Relatórios analíticos recursivos e comparativo lado a lado")
    ),
    info(
        title = "Player Scout API",
        version = "0.1.0",
        description = "API REST oficial do Player Scout para scouting em quadra, relatórios analíticos e integração com o aplicativo móvel Android/iOS em Kotlin."
    )
)]
pub struct ApiDoc;

pub fn app_router(db: Database) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Rotas públicas (Login, Cadastro, OAuth2 Authorize, Logoff e Healthcheck)
    let public_routes = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health::health_check))
        .route("/api/v1/health", get(health::health_check))
        .route("/web/login", get(web::login_page).post(web::web_login))
        .route("/web/register", get(web::register_page))
        .route("/web/partials/auth/login", post(web::web_login_partial))
        .route(
            "/web/partials/auth/register",
            post(web::web_register_partial),
        )
        .route("/web/auth/logout", post(web::web_logout))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route(
            "/api/v1/auth/oauth/{provider}/authorize",
            get(auth::oauth_authorize),
        )
        .route(
            "/api/v1/auth/oauth/{provider}/login",
            post(auth::oauth_login),
        )
        .route(
            "/web/auth/oauth/{provider}/authorize",
            get(web::web_oauth_authorize),
        )
        .route(
            "/web/auth/oauth/{provider}/callback",
            get(web::web_oauth_callback),
        );

    // Rotas protegidas por Sessão Única (12h Argon2i) + Defesa Anti-CSRF (`Sec-Fetch-Site`)
    let protected_routes = Router::new()
        .route("/", get(web::dashboard_page))
        .route("/web/dashboard", get(web::dashboard_page))
        .route("/web/matches/setup", get(web::match_setup_page))
        .route("/web/matches/new", post(web::create_match_web))
        .route("/web/matches/rematch", post(web::rematch_web))
        .route(
            "/web/matches/quick-player",
            post(web::create_quick_player_web),
        )
        .route("/web/court/match/finish", post(web::finish_match_web))
        .route("/web/court", get(web::court_page))
        .route("/web/partials/analytics", get(web::analytics_partial))
        .route("/web/partials/compare", get(web::compare_partial))
        .route(
            "/web/partials/court/shot",
            post(web::court_record_shot_partial),
        )
        .route(
            "/web/partials/court/undo",
            post(web::court_undo_shot_partial),
        )
        .route("/web/preferences/profile", post(web::web_update_profile))
        .route("/web/preferences/password", post(web::web_update_password))
        .route("/api/v1/auth/session", get(auth::session_status))
        .route("/api/v1/auth/password", post(auth::change_password))
        .route("/api/v1/auth/lookup", get(auth::lookup_user))
        .route(
            "/api/v1/auth/oauth/{provider}/callback",
            post(auth::oauth_callback),
        )
        .route(
            "/api/v1/users",
            post(users::create_user).get(users::list_users),
        )
        .route(
            "/api/v1/users/{id}",
            get(users::get_user).delete(users::deactivate_user),
        )
        .route(
            "/api/v1/users/{id}/reactivate",
            post(users::reactivate_user),
        )
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
        .route_layer(middleware::from_fn_with_state(
            db.clone(),
            auth::require_session_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(db)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
