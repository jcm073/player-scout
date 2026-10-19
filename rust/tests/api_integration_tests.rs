use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use player_scout::{
    AppConfig, Database,
    api::{
        analytics::CompareAnalyticsRequest,
        matches::CreateMatchRequest,
        players::{AssignCoachRequest, CreatePlayerRequest},
        shots::{RecordShotRequest, UndoShotRequest},
        users::CreateUserRequest,
    },
    create_app,
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, CoachPlayer, DecisionQuality, DominantHand,
        MatchSession, Player, PlayerAnalyticsReport, PreferredSide, ShotEvent, ShotOutcome,
        ShotType, SpecialTag, User, UserRole,
    },
    error::ErrorResponse,
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use uuid::Uuid;

static BOOTSTRAP_SESSION_TOKEN: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

async fn connect_test_db() -> Database {
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());
    let config = AppConfig::from_env().expect("DATABASE_URL deve estar configurada no .env");
    Database::connect(&config.database_url)
        .await
        .expect("Falha ao conectar ao PostgreSQL 18 via Toasty ORM")
}

async fn setup_test_app() -> axum::Router {
    let db = connect_test_db().await;
    let db_for_init = db.clone();
    let _ = BOOTSTRAP_SESSION_TOKEN
        .get_or_init(|| async move {
            let boot_user = db_for_init
                .create_user(
                    UserRole::Admin,
                    "Bootstrap Test Admin".to_string(),
                    format!("boot.admin.{}@padel.com", Uuid::now_v7()),
                    "argon2i_bootstrap_hash".to_string(),
                )
                .await
                .expect("Falha ao criar usuário de bootstrap para sessão dos testes");
            let (raw_token, _session) = db_for_init
                .create_or_replace_session(boot_user.id)
                .await
                .expect("Falha ao criar sessão de bootstrap nos testes");
            raw_token
        })
        .await;
    create_app(db)
}

async fn send_json<T: serde::Serialize, R: DeserializeOwned>(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<&T>,
) -> (StatusCode, R) {
    let token = BOOTSTRAP_SESSION_TOKEN
        .get()
        .map(String::as_str)
        .unwrap_or("");
    send_json_with_token(app, token, method, uri, body).await
}

async fn send_json_with_token<T: serde::Serialize, R: DeserializeOwned>(
    app: &axum::Router,
    token: &str,
    method: Method,
    uri: &str,
    body: Option<&T>,
) -> (StatusCode, R) {
    let payload = match body {
        Some(b) => serde_json::to_vec(b).expect("Falha ao serializar JSON"),
        None => Vec::new(),
    };

    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if !token.is_empty() {
        builder = builder
            .header("authorization", format!("Bearer {token}"))
            .header("cookie", format!("ps_session={token}"));
    }

    let req = builder
        .body(Body::from(payload))
        .expect("Falha ao construir requisição HTTP");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("Falha ao executar requisição no Router");

    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("Falha ao ler corpo da resposta");
    let parsed: R = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "Falha ao desserializar resposta JSON ({status}): {e} | Body: {}",
            String::from_utf8_lossy(&bytes)
        )
    });

    (status, parsed)
}

async fn send_get_html(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let token = BOOTSTRAP_SESSION_TOKEN
        .get()
        .map(String::as_str)
        .unwrap_or("");
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("cookie", format!("ps_session={token}"))
        .body(Body::empty())
        .expect("Falha ao construir requisição GET HTML");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("Falha ao executar requisição GET HTML");

    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("Falha ao ler corpo HTML");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn send_form_html(app: &axum::Router, uri: &str, form_body: &str) -> (StatusCode, String) {
    let token = BOOTSTRAP_SESSION_TOKEN
        .get()
        .map(String::as_str)
        .unwrap_or("");
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("authorization", format!("Bearer {token}"))
        .header("cookie", format!("ps_session={token}"))
        .body(Body::from(form_body.to_owned()))
        .expect("Falha ao construir requisição POST Form");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("Falha ao executar requisição POST Form");

    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("Falha ao ler corpo da resposta Form");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn test_end_to_end_toasty_persistence_rbac_and_shot_undo() {
    let app = setup_test_app().await;
    let unique_tag = Uuid::now_v7();

    // 1. Health Check
    let (status, health): (StatusCode, serde_json::Value) =
        send_json::<(), _>(&app, Method::GET, "/api/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["status"], "ok");
    assert_eq!(health["service"], "player-scout-backend");

    // 2. Criação de Usuários: Coach Técnico, Coach Tático e Usuário Atleta (Player Read-Only)
    let (status, coach_tech): (StatusCode, User) = send_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(&CreateUserRequest {
            role: UserRole::Coach,
            name: "Coach Técnico Padel".to_string(),
            email: format!("coach.tech.{unique_tag}@playerscout.test"),
            password_hash: "argon2_hash_placeholder".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(coach_tech.role, UserRole::Coach);

    let (status, coach_tactical): (StatusCode, User) = send_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(&CreateUserRequest {
            role: UserRole::Coach,
            name: "Coach Tático Padel".to_string(),
            email: format!("coach.tactical.{unique_tag}@playerscout.test"),
            password_hash: "argon2_hash_placeholder".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, athlete_user): (StatusCode, User) = send_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(&CreateUserRequest {
            role: UserRole::Player,
            name: "Atleta Lucas".to_string(),
            email: format!("athlete.lucas.{unique_tag}@playerscout.test"),
            password_hash: "argon2_hash_placeholder".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(athlete_user.role.is_read_only());

    // 3. RBAC: Perfil Player NÃO pode cadastrar jogadores (403 Forbidden)
    let (status, err_resp): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/players",
        Some(&CreatePlayerRequest {
            actor_id: athlete_user.id,
            user_id: None,
            name: "Tentativa Não Autorizada".to_string(),
            dominant_hand: DominantHand::RightHanded,
            preferred_side: PreferredSide::Left,
            category: Some("Pro".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(err_resp.code, "FORBIDDEN");

    // 4. Coach Técnico cadastra o Atleta Lucas (vinculado à conta athlete_user) e um segundo aluno
    let (status, player_lucas): (StatusCode, Player) = send_json(
        &app,
        Method::POST,
        "/api/v1/players",
        Some(&CreatePlayerRequest {
            actor_id: coach_tech.id,
            user_id: Some(athlete_user.id),
            name: "Lucas Silva".to_string(),
            dominant_hand: DominantHand::RightHanded,
            preferred_side: PreferredSide::Left,
            category: Some("1ª Categoria".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(player_lucas.user_id, Some(athlete_user.id));

    let (status, player_other): (StatusCode, Player) = send_json(
        &app,
        Method::POST,
        "/api/v1/players",
        Some(&CreatePlayerRequest {
            actor_id: coach_tech.id,
            user_id: None,
            name: "Mateus Outro Aluno".to_string(),
            dominant_hand: DominantHand::LeftHanded,
            preferred_side: PreferredSide::Right,
            category: Some("2ª Categoria".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // 5. Relacionamento N:N (Multi-Coach): atribui o Coach Tático ao Atleta Lucas
    let (status, _link): (StatusCode, CoachPlayer) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/players/{}/coaches", player_lucas.id),
        Some(&AssignCoachRequest {
            actor_id: coach_tech.id,
            coach_id: coach_tactical.id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, coaches): (StatusCode, Vec<User>) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!(
            "/api/v1/players/{}/coaches?actor_id={}",
            player_lucas.id, athlete_user.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(coaches.len(), 2);

    // 6. Isolamento de Privacidade: Atleta Lucas vê apenas o seu próprio perfil, e bloqueia acesso ao outro aluno
    let (status, visible_to_athlete): (StatusCode, Vec<Player>) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!("/api/v1/players?actor_id={}", athlete_user.id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(visible_to_athlete.len(), 1);
    assert_eq!(visible_to_athlete[0].id, player_lucas.id);

    let (status, forbidden_other): (StatusCode, ErrorResponse) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!(
            "/api/v1/players/{}?actor_id={}",
            player_other.id, athlete_user.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(forbidden_other.code, "FORBIDDEN");

    // 7. Criação de Partida e Ingestão de Jogadas em Quadra
    let (status, match_session): (StatusCode, MatchSession) = send_json(
        &app,
        Method::POST,
        "/api/v1/matches",
        Some(&CreateMatchRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            opponent_name: Some("Dupla Adversária Final".to_string()),
            notes: Some("Quadra rápida coberta".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Atleta não pode registrar jogadas (somente leitura)
    let (status, _): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: athlete_user.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::Saque,
            outcome: ShotOutcome::Hit,
            decision: None,
            special_tag: None,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Validação de domínio: Winner com Miss deve retornar 422 Unprocessable Entity
    let (status, domain_err): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::SmashX3,
            outcome: ShotOutcome::Miss,
            decision: Some(DecisionQuality::Good),
            special_tag: Some(SpecialTag::Winner),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(domain_err.code, "WINNER_MUST_BE_HIT");

    // Registro de 3 jogadas válidas pelo Coach Tático
    let (status, shot1): (StatusCode, ShotEvent) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::Vibora,
            outcome: ShotOutcome::Hit,
            decision: None, // Deve assumir Good por padrão
            special_tag: None,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(shot1.decision, DecisionQuality::Good);

    let (status, shot2): (StatusCode, ShotEvent) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::SmashX3,
            outcome: ShotOutcome::Hit,
            decision: Some(DecisionQuality::Good),
            special_tag: Some(SpecialTag::Winner),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(shot2.special_tag, Some(SpecialTag::Winner));

    let (status, shot3): (StatusCode, ShotEvent) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::Bandeja,
            outcome: ShotOutcome::Miss,
            decision: Some(DecisionQuality::Bad),
            special_tag: Some(SpecialTag::UnforcedError),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // 8. Undo da Última Jogada: desfaz shot3 (Bandeja)
    let (status, undone): (StatusCode, ShotEvent) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots/undo", match_session.id),
        Some(&UndoShotRequest {
            actor_id: coach_tactical.id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(undone.id, shot3.id);
    assert_eq!(undone.shot_type, ShotType::Bandeja);

    // 9. Consulta do Histórico da Partida pelo próprio Atleta (Read-Only)
    let (status, remaining_shots): (StatusCode, Vec<ShotEvent>) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!(
            "/api/v1/matches/{}/shots?actor_id={}",
            match_session.id, athlete_user.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(remaining_shots.len(), 2);
    assert_eq!(remaining_shots[0].id, shot1.id);
    assert_eq!(remaining_shots[1].id, shot2.id);

    // 10. Registra uma jogada no Set 1 (erro) e duas jogadas no Set 2 para testar o Motor Analítico e Comparativo
    let (status, _shot_s1_miss): (StatusCode, ShotEvent) = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/matches/{}/shots", match_session.id),
        Some(&RecordShotRequest {
            actor_id: coach_tactical.id,
            player_id: player_lucas.id,
            set_number: 1,
            shot_type: ShotType::BajadaParede,
            outcome: ShotOutcome::Miss,
            decision: Some(DecisionQuality::Bad),
            special_tag: Some(SpecialTag::UnforcedError),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Set 2: 2 jogadas perfeitas (100% hit rate no Set 2 vs 66.67% no Set 1)
    for shot_type in [ShotType::Saque, ShotType::VoleioDireita] {
        let (status, _): (StatusCode, ShotEvent) = send_json(
            &app,
            Method::POST,
            &format!("/api/v1/matches/{}/shots", match_session.id),
            Some(&RecordShotRequest {
                actor_id: coach_tactical.id,
                player_id: player_lucas.id,
                set_number: 2,
                shot_type,
                outcome: ShotOutcome::Hit,
                decision: Some(DecisionQuality::Good),
                special_tag: None,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // 11. Agregação Recursiva: Consulta Geral da Partida pelo Atleta (5 jogadas totais: 4 hits, 1 miss = 80%)
    let (status, analytics_match): (StatusCode, PlayerAnalyticsReport) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!(
            "/api/v1/analytics/players/{}?actor_id={}&match_id={}",
            player_lucas.id, athlete_user.id, match_session.id
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(analytics_match.total_shots, 5);
    assert_eq!(analytics_match.hits, 4);
    assert_eq!(analytics_match.misses, 1);
    assert_eq!(analytics_match.hit_rate_pct, 80.0);
    assert_eq!(analytics_match.matrix.hit_good, 4);
    assert_eq!(analytics_match.matrix.miss_bad, 1);
    assert_eq!(analytics_match.winners, 1);
    assert_eq!(analytics_match.unforced_errors, 1);
    assert_eq!(analytics_match.winner_to_ue_balance, 0);

    // 12. Modo Comparativo Lado a Lado: Set 1 (66.67% acerto) vs Set 2 (100% acerto)
    let (status, comparison): (StatusCode, AnalyticsComparisonReport) = send_json(
        &app,
        Method::POST,
        "/api/v1/analytics/compare",
        Some(&CompareAnalyticsRequest {
            actor_id: athlete_user.id,
            player_id: player_lucas.id,
            scope_a: AnalyticsFilter {
                match_id: Some(match_session.id),
                set_number: Some(1),
                from_date: None,
                to_date: None,
            },
            scope_b: AnalyticsFilter {
                match_id: Some(match_session.id),
                set_number: Some(2),
                from_date: None,
                to_date: None,
            },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(comparison.scope_a.total_shots, 3);
    assert_eq!(comparison.scope_a.hit_rate_pct, 66.67);
    assert_eq!(comparison.scope_b.total_shots, 2);
    assert_eq!(comparison.scope_b.hit_rate_pct, 100.0);
    assert_eq!(comparison.deltas.hit_rate_delta_pct, 33.33);
    assert_eq!(comparison.deltas.good_decision_rate_delta_pct, 33.33);
}

#[tokio::test]
async fn test_web_askama_htmx_dashboard_and_touch_first_court_flow() {
    let app = setup_test_app().await;
    let unique_tag = Uuid::now_v7();

    // 1. Verifica renderização inicial do Dashboard sem query params (Modo Demonstração)
    let (status, demo_html) = send_get_html(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(demo_html.contains("Player Scout"));
    assert!(demo_html.contains("chart.js@4.4.9"));
    assert!(demo_html.contains("Matriz Tática: Decisão × Execução"));
    assert!(demo_html.contains("padelRadarChart"));

    // 2. Cria Treinador, Atleta e Partida para validar fluxo completo HTMX
    let (status, coach): (StatusCode, User) = send_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(&CreateUserRequest {
            role: UserRole::Coach,
            name: "Coach Web HTMX".to_string(),
            email: format!("coach.web.{unique_tag}@playerscout.test"),
            password_hash: "argon2_hash_placeholder".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, athlete_user): (StatusCode, User) = send_json(
        &app,
        Method::POST,
        "/api/v1/users",
        Some(&CreateUserRequest {
            role: UserRole::Player,
            name: "Sofia Padel Pro".to_string(),
            email: format!("sofia.web.{unique_tag}@playerscout.test"),
            password_hash: "argon2_hash_placeholder".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, player): (StatusCode, Player) = send_json(
        &app,
        Method::POST,
        "/api/v1/players",
        Some(&CreatePlayerRequest {
            actor_id: coach.id,
            user_id: Some(athlete_user.id),
            name: "Sofia Padel Pro".to_string(),
            dominant_hand: DominantHand::RightHanded,
            preferred_side: PreferredSide::Right,
            category: Some("1ª Categoria".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, match_session): (StatusCode, MatchSession) = send_json(
        &app,
        Method::POST,
        "/api/v1/matches",
        Some(&CreateMatchRequest {
            actor_id: coach.id,
            player_id: player.id,
            opponent_name: Some("Dupla Rival Final".to_string()),
            notes: Some("Quadra Central Panorâmica".to_string()),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // 3. Verifica Tela Touch-First de Quadra (`GET /web/court`)
    let (status, court_html) = send_get_html(
        &app,
        &format!(
            "/web/court?actor_id={}&player_id={}&match_id={}",
            coach.id, player.id, match_session.id
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(court_html.contains("Sofia Padel Pro"));
    assert!(court_html.contains("ACERTOU (IN)"));
    assert!(court_html.contains("ERROU (OUT)"));
    assert!(court_html.contains("Desfazer (Undo)"));
    assert!(court_html.contains("navigator.vibrate"));
    assert!(court_html.contains("min-h-[48px]"));
    assert!(court_html.contains("min-h-[64px]"));
    // Confere presença das 3 zonas táticas e golpes do catálogo de Padel
    assert!(court_html.contains("Início de Ponto"));
    assert!(court_html.contains("Rede &amp; Ataque") || court_html.contains("Rede & Ataque"));
    assert!(court_html.contains("Fundo &amp; Vidros") || court_html.contains("Fundo & Vidros"));
    assert!(court_html.contains("Víbora"));
    assert!(court_html.contains("Bajada de Parede"));

    // 4. Registro de Golpe via HTMX Form (`POST /web/partials/court/shot`) — Winner no Set 1
    let form_shot_1 = format!(
        "actor_id={}&player_id={}&match_id={}&set_number=1&shot_type=vibora&outcome=hit&decision=good&special_tag=winner",
        coach.id, player.id, match_session.id
    );
    let (status, feed_html_1) =
        send_form_html(&app, "/web/partials/court/shot", &form_shot_1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(feed_html_1.contains("Golpe registrado: Víbora (ACERTOU) — Set 1"));
    assert!(feed_html_1.contains("100%"));

    // 5. Testa regra de ergonomia Touch-First: marcar `unforced_error` força automaticamente `Miss`
    let form_shot_2 = format!(
        "actor_id={}&player_id={}&match_id={}&set_number=1&shot_type=bandeja&outcome=hit&decision=bad&special_tag=unforced_error",
        coach.id, player.id, match_session.id
    );
    let (status, feed_html_2) =
        send_form_html(&app, "/web/partials/court/shot", &form_shot_2).await;
    assert_eq!(status, StatusCode::OK);
    assert!(feed_html_2.contains("Golpe registrado: Bandeja (ERROU) — Set 1"));
    assert!(feed_html_2.contains("50%"));

    // 6. RBAC na rota HTMX: Jogador (`Player` Read-Only) não pode registrar golpe nem desfazer
    let forbidden_form = format!(
        "actor_id={}&player_id={}&match_id={}&set_number=1&shot_type=saque&outcome=hit&decision=good&special_tag=",
        athlete_user.id, player.id, match_session.id
    );
    let (status, _) = send_form_html(&app, "/web/partials/court/shot", &forbidden_form).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // 7. Desfazer Última Jogada via HTMX (`POST /web/partials/court/undo`)
    let undo_form = format!("actor_id={}&match_id={}", coach.id, match_session.id);
    let (status, feed_undo_html) =
        send_form_html(&app, "/web/partials/court/undo", &undo_form).await;
    assert_eq!(status, StatusCode::OK);
    assert!(feed_undo_html.contains("Jogada desfeita: Bandeja (Set 1)"));
    assert!(feed_undo_html.contains("100%"));

    // 8. Registra golpe no Set 2 e valida parciais HTMX do Dashboard (`analytics` e `compare`)
    let form_shot_set2 = format!(
        "actor_id={}&player_id={}&match_id={}&set_number=2&shot_type=smash_x3&outcome=hit&decision=good&special_tag=winner",
        coach.id, player.id, match_session.id
    );
    let (status, _) = send_form_html(&app, "/web/partials/court/shot", &form_shot_set2).await;
    assert_eq!(status, StatusCode::OK);

    let (status, partial_analytics) = send_get_html(
        &app,
        &format!(
            "/web/partials/analytics?actor_id={}&player_id={}&match_id={}",
            athlete_user.id, player.id, match_session.id
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(partial_analytics.contains("id=\"analytics-panel-content\""));
    assert!(partial_analytics.contains("Smash x3"));
    assert!(partial_analytics.contains("Víbora"));

    let (status, partial_compare) = send_get_html(
        &app,
        &format!(
            "/web/partials/compare?actor_id={}&player_id={}&match_id={}&set_a=1&set_b=2",
            athlete_user.id, player.id, match_session.id
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(partial_compare.contains("id=\"compare-panel-content\""));
    assert!(partial_compare.contains("Modo Comparativo Lado a Lado (Set 1 vs Set 2)"));

    // 9. Dashboard completo autenticado (`GET /web/dashboard`)
    let (status, full_dashboard) = send_get_html(
        &app,
        &format!(
            "/web/dashboard?actor_id={}&player_id={}&match_id={}",
            athlete_user.id, player.id, match_session.id
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(full_dashboard.contains("Sofia Padel Pro"));
    assert!(full_dashboard.contains("Modo Comparativo Lado a Lado (Set 1 vs Set 2)"));
}

/// Gera um CPF matematicamente válido (Módulo 11) único a partir de uma semente de 9 dígitos
fn generate_unique_valid_cpf(seed: u64) -> String {
    let base_num = 100_000_000 + (seed % 899_000_000);
    let mut digits: Vec<u32> = format!("{base_num:09}")
        .chars()
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    // Evita que todos os 9 dígitos sejam iguais
    if digits.iter().all(|&d| d == digits[0]) {
        digits[8] = (digits[0] + 1) % 10;
    }

    let sum1: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (10 - i as u32))
        .sum();
    let rem1 = (sum1 * 10) % 11;
    let dv1 = if rem1 == 10 { 0 } else { rem1 };
    digits.push(dv1);

    let sum2: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (11 - i as u32))
        .sum();
    let rem2 = (sum2 * 10) % 11;
    let dv2 = if rem2 == 10 { 0 } else { rem2 };
    digits.push(dv2);

    digits
        .into_iter()
        .map(|d| char::from_digit(d, 10).unwrap())
        .collect()
}

#[tokio::test]
async fn test_phase_4_5_registration_argon2i_login_cpf_lookup_and_oauth() {
    use player_scout::{
        api::auth::{
            AuthResponse, LoginRequest, OAuthAuthorizeResponse, OAuthCallbackRequest,
            OAuthLoginRequest, OAuthLoginResponse,
        },
        domain::{OAuthProvider, RegistrationInput, UserOAuthAccount},
    };

    let app = setup_test_app().await;
    let unique_uuid = Uuid::now_v7();
    let seed = (unique_uuid.as_u128() & 0xFFFF_FFFF) as u64;
    let player_cpf = generate_unique_valid_cpf(seed);
    let coach_cpf = generate_unique_valid_cpf(seed + 137);

    // 1. Verifica Telas Web de Login e Cadastro (`GET /web/login` e `GET /web/register`)
    let (status, login_html) = send_get_html(&app, "/web/login").await;
    assert_eq!(status, StatusCode::OK);
    assert!(login_html.contains("Acesso exclusivo por E-mail · Conexão Segura"));
    assert!(!login_html.contains("Argon2i"));
    assert!(login_html.contains("togglePasswordVisibility"));
    assert!(login_html.contains("Continuar com Google (Gmail)"));
    assert!(login_html.contains("Continuar com Microsoft Office"));
    assert!(login_html.contains("Continuar com Instagram"));

    let (status, register_html) = send_get_html(&app, "/web/register").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        register_html.contains("Cadastro Completo · Player &amp; Coach")
            || register_html.contains("Cadastro Completo · Player & Coach")
    );
    assert!(!register_html.contains("Argon2i"));
    assert!(register_html.contains("togglePasswordVisibility"));
    assert!(register_html.contains("Confirme sua Senha"));
    assert!(register_html.contains("onConfirmPasswordInput"));
    assert!(register_html.contains("Posição em Quadra"));
    assert!(register_html.contains("Esquerda / Revés"));
    assert!(register_html.contains("Direita / Drive"));
    assert!(register_html.contains("isValidCpfModulo11"));
    assert!(register_html.contains("onComplementInput"));
    assert!(register_html.contains("onPasswordInput"));
    assert!(register_html.contains("validateRegistrationForm"));
    assert!(register_html.contains("pw-rule-len"));
    assert!(register_html.contains("pw-rule-special"));
    assert!(register_html.contains("maxlength=\"10\""));
    assert!(register_html.contains("minlength=\"12\""));

    // 2. Cadastro Completo de um Atleta (Player) com criação automática do perfil em `players`
    let player_email = format!("atleta.auth.{unique_uuid}@playerscout.test");
    let register_player_req = RegistrationInput {
        first_name: "Beatriz".to_string(),
        last_name: "González".to_string(),
        age: 23,
        cpf: player_cpf.clone(),
        email: player_email.clone(),
        whatsapp_phone: "(51) 99888-7766".to_string(),
        cep: "90570-020".to_string(),
        street: "Rua Padre Chagas".to_string(),
        street_number: "320".to_string(),
        complement: "Apto 402".to_string(),
        neighborhood: "Moinhos de Vento".to_string(),
        city: "Porto Alegre".to_string(),
        state: "rs".to_string(),
        role: UserRole::Player,
        preferred_side: PreferredSide::Left,
        dominant_hand: DominantHand::RightHanded,
        password: "PadelMaster2026!#".to_string(),
    };

    let (status, player_auth): (StatusCode, AuthResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(&register_player_req),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(player_auth.user.name, "Beatriz González");
    assert_eq!(player_auth.user.first_name.as_deref(), Some("Beatriz"));
    assert_eq!(player_auth.user.last_name.as_deref(), Some("González"));
    assert_eq!(player_auth.user.age, Some(23));
    assert_eq!(player_auth.user.cpf.as_deref(), Some(player_cpf.as_str()));
    assert_eq!(player_auth.user.cep.as_deref(), Some("90570020"));
    assert_eq!(player_auth.user.complement.as_deref(), Some("Apto 402"));
    assert_eq!(player_auth.user.state.as_deref(), Some("RS"));
    assert_eq!(player_auth.user.preferred_side, PreferredSide::Left);

    // Confirma que o perfil esportivo na tabela `players` foi criado automaticamente
    let auto_player = player_auth
        .player_profile
        .expect("O perfil de atleta na tabela players deve ser criado automaticamente");
    assert_eq!(auto_player.user_id, Some(player_auth.user.id));
    assert_eq!(auto_player.preferred_side, PreferredSide::Left);

    // 3. Cadastro Completo de um Treinador (Coach) jogando na Direita (Drive) e sem complemento (casa)
    let coach_email = format!("coach.auth.{unique_uuid}@playerscout.test");
    let register_coach_req = RegistrationInput {
        first_name: "Ramiro".to_string(),
        last_name: "Choya".to_string(),
        age: 45,
        cpf: coach_cpf.clone(),
        email: coach_email.clone(),
        whatsapp_phone: "11977776666".to_string(),
        cep: "01310-100".to_string(),
        street: "Av. Paulista".to_string(),
        street_number: "1000".to_string(),
        complement: "".to_string(),
        neighborhood: "Bela Vista".to_string(),
        city: "São Paulo".to_string(),
        state: "SP".to_string(),
        role: UserRole::Coach,
        preferred_side: PreferredSide::Right,
        dominant_hand: DominantHand::RightHanded,
        password: "CoachSecure2026$%".to_string(),
    };

    let (status, coach_auth): (StatusCode, AuthResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(&register_coach_req),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(coach_auth.user.role, UserRole::Coach);
    assert_eq!(coach_auth.user.preferred_side, PreferredSide::Right);
    assert!(coach_auth.user.complement.is_none());
    assert!(coach_auth.player_profile.is_none());

    // 4. Bloqueio de CPF duplicado (409 Conflict)
    let mut dup_cpf_req = register_coach_req.clone();
    dup_cpf_req.email = format!("outro.{unique_uuid}@playerscout.test");
    let (status, conflict_err): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(&dup_cpf_req),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict_err.code, "CONFLICT");

    // 5. Login exclusivo por E-mail com verificação Argon2i (`POST /api/v1/auth/login`)
    let (status, login_ok): (StatusCode, AuthResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_email.clone(),
            password: "PadelMaster2026!#".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(login_ok.user.id, player_auth.user.id);
    assert!(login_ok.player_profile.is_some());

    // Senha incorreta -> 401 Unauthorized
    let (status, login_fail): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_email.clone(),
            password: "SenhaErrada2026!#".to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(login_fail.code, "UNAUTHORIZED");

    // 6. Busca de usuário por chave única: CPF ou E-mail (`GET /api/v1/auth/lookup`)
    let (status, found_by_cpf): (StatusCode, User) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!("/api/v1/auth/lookup?cpf={player_cpf}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(found_by_cpf.id, player_auth.user.id);

    let (status, found_by_email): (StatusCode, User) = send_json::<(), _>(
        &app,
        Method::GET,
        &format!("/api/v1/auth/lookup?email={coach_email}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(found_by_email.id, coach_auth.user.id);

    // 7. Fluxo OAuth2 Homologado (Google/Gmail, Microsoft Office e Instagram)
    for provider in ["google", "microsoft", "instagram"] {
        let (status, auth_url_res): (StatusCode, OAuthAuthorizeResponse) = send_json::<(), _>(
            &app,
            Method::GET,
            &format!("/api/v1/auth/oauth/{provider}/authorize"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(auth_url_res.authorize_url.starts_with("https://"));
        assert!(auth_url_res.token_url.starts_with("https://"));
        assert!(auth_url_res.userinfo_url.starts_with("https://"));
    }

    let (status, oauth_linked): (StatusCode, UserOAuthAccount) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/oauth/google/callback",
        Some(&OAuthCallbackRequest {
            user_id: player_auth.user.id,
            provider_user_id: format!("google-uid-{unique_uuid}"),
            provider_email: player_email.clone(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(oauth_linked.provider, OAuthProvider::Google);
    assert_eq!(oauth_linked.user_id, player_auth.user.id);

    // 8. Homologação E2E de Login Social OAuth2 com Anti-CSRF (`ps_oauth_state`) e Sessão Única 12h
    let authz_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/oauth/google/authorize")
        .body(Body::empty())
        .unwrap();
    let authz_res = app.clone().oneshot(authz_req).await.unwrap();
    assert_eq!(authz_res.status(), StatusCode::OK);
    let set_cookie_header = authz_res
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie_header.contains("ps_oauth_state="));
    let authz_bytes = axum::body::to_bytes(authz_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let authz_body: OAuthAuthorizeResponse = serde_json::from_slice(&authz_bytes).unwrap();
    let valid_state = authz_body.state;

    // 8a. Rejeita tentativa de Login OAuth2 com state Anti-CSRF divergente do cookie (403 Forbidden)
    let csrf_fail_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/oauth/google/login")
        .header("content-type", "application/json")
        .header("cookie", format!("ps_oauth_state={valid_state}"))
        .body(Body::from(
            serde_json::to_vec(&OAuthLoginRequest {
                provider_user_id: format!("google-uid-{unique_uuid}"),
                provider_email: player_email.clone(),
                state: Some("state-forjado-atacante".to_string()),
                code: Some("auth_code_123".to_string()),
            })
            .unwrap(),
        ))
        .unwrap();
    let csrf_fail_res = app.clone().oneshot(csrf_fail_req).await.unwrap();
    assert_eq!(csrf_fail_res.status(), StatusCode::FORBIDDEN);

    // 8b. Login OAuth2 válido emite novo token de sessão única de 12h (`ps_sess_...` Argon2i)
    let oauth_login_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/oauth/google/login")
        .header("content-type", "application/json")
        .header("cookie", format!("ps_oauth_state={valid_state}"))
        .body(Body::from(
            serde_json::to_vec(&OAuthLoginRequest {
                provider_user_id: format!("google-uid-{unique_uuid}"),
                provider_email: player_email.clone(),
                state: Some(valid_state.clone()),
                code: Some("auth_code_123".to_string()),
            })
            .unwrap(),
        ))
        .unwrap();
    let oauth_login_res = app.clone().oneshot(oauth_login_req).await.unwrap();
    assert_eq!(oauth_login_res.status(), StatusCode::OK);
    let oauth_session_cookie = oauth_login_res
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(oauth_session_cookie.contains("ps_session=ps_sess_"));
    let oauth_login_bytes = axum::body::to_bytes(oauth_login_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let oauth_login_body: OAuthLoginResponse = serde_json::from_slice(&oauth_login_bytes).unwrap();
    assert_eq!(oauth_login_body.user.id, player_auth.user.id);
    assert!(oauth_login_body.session_token.starts_with("ps_sess_"));

    // 8c. Fluxo Web OAuth2 (`/web/auth/oauth/google/authorize` e `/web/auth/oauth/google/callback`)
    let web_authz_req = Request::builder()
        .method(Method::GET)
        .uri("/web/auth/oauth/google/authorize")
        .body(Body::empty())
        .unwrap();
    let web_authz_res = app.clone().oneshot(web_authz_req).await.unwrap();
    assert_eq!(web_authz_res.status(), StatusCode::SEE_OTHER);
    assert!(
        web_authz_res
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("https://accounts.google.com/o/oauth2/v2/auth")
    );

    // Callback Web para e-mail ainda não cadastrado -> redireciona para `/web/register?email=...`
    let web_cb_new_req = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/web/auth/oauth/google/callback?code=c1&state={valid_state}&email=novo.oauth.{unique_uuid}@playerscout.test"
        ))
        .header("cookie", format!("ps_oauth_state={valid_state}"))
        .body(Body::empty())
        .unwrap();
    let web_cb_new_res = app.clone().oneshot(web_cb_new_req).await.unwrap();
    assert_eq!(web_cb_new_res.status(), StatusCode::SEE_OTHER);
    assert!(
        web_cb_new_res
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("/web/register?email=novo.oauth.")
    );

    // Callback Web para usuário já cadastrado -> emite cookie `ps_session` (12h) e redireciona para `/web/dashboard`
    let web_cb_ok_req = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/web/auth/oauth/microsoft/callback?code=ms_code&state={valid_state}&email={coach_email}&provider_user_id=ms-uid-{unique_uuid}"
        ))
        .header("cookie", format!("ps_oauth_state={valid_state}"))
        .body(Body::empty())
        .unwrap();
    let web_cb_ok_res = app.clone().oneshot(web_cb_ok_req).await.unwrap();
    assert_eq!(web_cb_ok_res.status(), StatusCode::SEE_OTHER);
    assert!(
        web_cb_ok_res
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("/web/dashboard?actor_id=")
    );
    assert!(
        web_cb_ok_res
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("ps_session=ps_sess_")
    );
}

#[tokio::test]
async fn test_session_12h_single_session_logout_expiration_and_brute_force_protection() {
    use chrono::Utc;
    use player_scout::{
        api::auth::{AuthResponse, LoginRequest, LogoutResponse},
        db::models::UserSessionRecord,
        domain::{AuthenticatedUser, RegistrationInput, SESSION_TTL_SECONDS},
    };

    let app = setup_test_app().await;
    let db = connect_test_db().await;

    // 1. Qualquer página ou endpoint protegido deve ser bloqueado sem token de sessão
    let unauth_web_req = Request::builder()
        .method(Method::GET)
        .uri("/web/dashboard")
        .body(Body::empty())
        .unwrap();
    let unauth_web_res = app.clone().oneshot(unauth_web_req).await.unwrap();
    assert_eq!(unauth_web_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        unauth_web_res
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/web/login?reason=session_required"
    );

    let (unauth_api_status, unauth_api_err): (StatusCode, ErrorResponse) =
        send_json_with_token(&app, "", Method::GET, "/api/v1/users", None::<&()>).await;
    assert_eq!(unauth_api_status, StatusCode::UNAUTHORIZED);
    assert_eq!(unauth_api_err.code, "UNAUTHORIZED");

    // 2. Cadastra um usuário e verifica que o token de sessão dura 12 horas (43.200s)
    let unique_uuid = Uuid::now_v7();
    let seed = ((unique_uuid.as_u128() >> 16) & 0xFFFF_FFFF) as u64 + 50_000;
    let email = format!("sessao.unica.{unique_uuid}@playerscout.test");
    let password = "Padel#2026StrongPass".to_string();

    let reg_input = RegistrationInput {
        first_name: "Arturo".to_string(),
        last_name: "Coello".to_string(),
        age: 24,
        cpf: generate_unique_valid_cpf(seed),
        email: email.clone(),
        whatsapp_phone: "(51) 99888-1122".to_string(),
        cep: "90010-150".to_string(),
        street: "Rua dos Andradas".to_string(),
        street_number: "500".to_string(),
        complement: "Apto 12".to_string(),
        neighborhood: "Centro".to_string(),
        city: "Porto Alegre".to_string(),
        state: "RS".to_string(),
        role: UserRole::Coach,
        preferred_side: PreferredSide::Right,
        dominant_hand: DominantHand::LeftHanded,
        password: password.clone(),
    };

    let (status, auth_1): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/register",
        Some(&reg_input),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(auth_1.session_token.starts_with("ps_sess_"));
    assert_eq!(
        auth_1.session.expires_at_epoch - auth_1.session.created_at_epoch,
        SESSION_TTL_SECONDS
    );

    // Token 1 acessa `/api/v1/auth/session` com sucesso
    let (status, sess_info): (StatusCode, AuthenticatedUser) = send_json_with_token(
        &app,
        &auth_1.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sess_info.user.id, auth_1.user.id);

    // 3. Sessão Única por Usuário: novo login revoga e exclui fisicamente a sessão anterior
    let (status, auth_2): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: email.clone(),
            password: password.clone(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(auth_1.session_token, auth_2.session_token);
    assert_ne!(auth_1.session.id, auth_2.session.id);

    // Token 1 antigo agora deve ser rejeitado (401 Unauthorized)
    let (status_old, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &auth_1.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(status_old, StatusCode::UNAUTHORIZED);

    // Token 2 novo funciona normalmente
    let (status_new, _): (StatusCode, AuthenticatedUser) = send_json_with_token(
        &app,
        &auth_2.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(status_new, StatusCode::OK);

    // 4. Logoff (`POST /api/v1/auth/logout`) exclui fisicamente a sessão do banco de dados
    let (status_logout, logout_res): (StatusCode, LogoutResponse) = send_json_with_token(
        &app,
        &auth_2.session_token,
        Method::POST,
        "/api/v1/auth/logout",
        None::<&()>,
    )
    .await;
    assert_eq!(status_logout, StatusCode::OK);
    assert!(logout_res.revoked);

    // Confirma que o registro foi excluído do PostgreSQL e o Token 2 não acessa mais nada
    let mut raw_db = db.raw();
    assert!(
        UserSessionRecord::get_by_id(&mut raw_db, &auth_2.session.id)
            .await
            .is_err(),
        "O token deve ser excluído fisicamente do banco no logoff"
    );
    let (status_after_logout, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &auth_2.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(status_after_logout, StatusCode::UNAUTHORIZED);

    // 5. Expiração de 12 horas: quando `expires_at_epoch <= now`, a sessão é excluída do banco (`DELETE`)
    let (status, auth_3): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: email.clone(),
            password: password.clone(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Simula passagem de 12 horas no registro da sessão no banco
    let mut sess_rec = UserSessionRecord::get_by_id(&mut raw_db, &auth_3.session.id)
        .await
        .expect("Sessão recém-criada deve existir");
    let past_epoch = Utc::now().timestamp() - 10;
    sess_rec
        .update()
        .created_at_epoch(past_epoch - 43_200)
        .expires_at_epoch(past_epoch)
        .exec(&mut raw_db)
        .await
        .expect("Deve atualizar expires_at_epoch para testar expiração");

    // Ao tentar usar a sessão expirada, recebe 401 e o registro é fisicamente excluído do banco
    let (status_expired, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &auth_3.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(status_expired, StatusCode::UNAUTHORIZED);
    assert!(
        UserSessionRecord::get_by_id(&mut raw_db, &auth_3.session.id)
            .await
            .is_err(),
        "A sessão expirada deve ser fisicamente excluída do banco de dados"
    );

    // 6. Proteção contra Força Bruta: 5 falhas consecutivas bloqueiam a conta por 15 minutos (429 Too Many Requests)
    for attempt in 1..=4 {
        let (st, err): (StatusCode, ErrorResponse) = send_json_with_token(
            &app,
            "",
            Method::POST,
            "/api/v1/auth/login",
            Some(&LoginRequest {
                email: email.clone(),
                password: format!("SenhaIncorreta{attempt}!@#"),
            }),
        )
        .await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "Tentativa {attempt} deve retornar 401"
        );
        // Mensagem uniforme para mitigar enumeração de contas
        assert!(err.error.contains("E-mail ou senha inválidos"));
    }

    // 5ª tentativa falha dispara o bloqueio de 15 minutos (429 Too Many Requests)
    let (st_5, err_5): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: email.clone(),
            password: "SenhaIncorreta5!@#".to_string(),
        }),
    )
    .await;
    assert_eq!(st_5, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(err_5.code, "TOO_MANY_REQUESTS");

    // Mesmo informando a senha CORRETA durante a janela de bloqueio de 15 minutos, o acesso permanece bloqueado (429)
    let (st_locked, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: email.clone(),
            password: password.clone(),
        }),
    )
    .await;
    assert_eq!(st_locked, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_fase_4_7_security_rbac_bola_lgpd_password_rotation_and_csrf() {
    use player_scout::{
        api::auth::{AuthResponse, ChangePasswordRequest, ChangePasswordResponse, LoginRequest},
        domain::RegistrationInput,
    };

    let app = setup_test_app().await;
    let unique_uuid = Uuid::now_v7();
    let seed = ((unique_uuid.as_u128() >> 24) & 0xFFFF_FFFF) as u64 + 90_000;

    // 1. Item 3.1: Bloqueio de escalação de privilégio no auto-cadastro (`role: Admin` -> 422 FORBIDDEN_ROLE_IN_REGISTRATION)
    let admin_reg = RegistrationInput {
        first_name: "Hacker".to_string(),
        last_name: "Admin".to_string(),
        age: 30,
        cpf: generate_unique_valid_cpf(seed),
        email: format!("hacker.admin.{unique_uuid}@playerscout.test"),
        whatsapp_phone: "(11) 99999-0000".to_string(),
        cep: "01310-100".to_string(),
        street: "Av Paulista".to_string(),
        street_number: "100".to_string(),
        complement: "Apto 1".to_string(),
        neighborhood: "Centro".to_string(),
        city: "São Paulo".to_string(),
        state: "SP".to_string(),
        role: UserRole::Admin,
        preferred_side: PreferredSide::Left,
        dominant_hand: DominantHand::RightHanded,
        password: "Padel#2026StrongPass".to_string(),
    };
    let (st_admin_reg, err_admin_reg): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/register",
        Some(&admin_reg),
    )
    .await;
    assert_eq!(st_admin_reg, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err_admin_reg.code, "FORBIDDEN_ROLE_IN_REGISTRATION");

    // 2. Cadastra um Player A e um Coach B para validar BOLA (Anti-Spoofing) e LGPD
    let player_a_email = format!("player.a.{unique_uuid}@playerscout.test");
    let player_a_pass = "Padel#2026PlayerAPass".to_string();
    let (_, player_a_auth): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/register",
        Some(&RegistrationInput {
            first_name: "Paula".to_string(),
            last_name: "Josemaría".to_string(),
            age: 27,
            cpf: generate_unique_valid_cpf(seed + 1),
            email: player_a_email.clone(),
            whatsapp_phone: "(11) 98888-1111".to_string(),
            cep: "01310-100".to_string(),
            street: "Av Paulista".to_string(),
            street_number: "200".to_string(),
            complement: "Bloco B".to_string(),
            neighborhood: "Bela Vista".to_string(),
            city: "São Paulo".to_string(),
            state: "SP".to_string(),
            role: UserRole::Player,
            preferred_side: PreferredSide::Right,
            dominant_hand: DominantHand::LeftHanded,
            password: player_a_pass.clone(),
        }),
    )
    .await;

    let coach_b_email = format!("coach.b.{unique_uuid}@playerscout.test");
    let (_, coach_b_auth): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/register",
        Some(&RegistrationInput {
            first_name: "Gustavo".to_string(),
            last_name: "Pratto".to_string(),
            age: 45,
            cpf: generate_unique_valid_cpf(seed + 2),
            email: coach_b_email.clone(),
            whatsapp_phone: "(11) 97777-2222".to_string(),
            cep: "01310-100".to_string(),
            street: "Av Paulista".to_string(),
            street_number: "300".to_string(),
            complement: "Sala 10".to_string(),
            neighborhood: "Bela Vista".to_string(),
            city: "São Paulo".to_string(),
            state: "SP".to_string(),
            role: UserRole::Coach,
            preferred_side: PreferredSide::Left,
            dominant_hand: DominantHand::RightHanded,
            password: "Padel#2026CoachBPass".to_string(),
        }),
    )
    .await;

    // 3. Item 3.2 (Anti-Spoofing / BOLA): Player A autenticado tenta se passar pelo Coach B enviando `actor_id = coach_b.id` -> 403 Forbidden
    let (st_spoof, err_spoof): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        &format!("/api/v1/players?actor_id={}", coach_b_auth.user.id),
        None::<&()>,
    )
    .await;
    assert_eq!(st_spoof, StatusCode::FORBIDDEN);
    assert_eq!(err_spoof.code, "FORBIDDEN");

    // 4. Item 3.3 (Privacidade / LGPD): Player A não pode listar todos os usuários, não pode usar `/auth/lookup`, nem ler o perfil cadastral do Coach B
    let (st_list_users, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        "/api/v1/users",
        None::<&()>,
    )
    .await;
    assert_eq!(st_list_users, StatusCode::FORBIDDEN);

    let (st_lookup, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        &format!("/api/v1/auth/lookup?email={coach_b_email}"),
        None::<&()>,
    )
    .await;
    assert_eq!(st_lookup, StatusCode::FORBIDDEN);

    let (st_other_user, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        &format!("/api/v1/users/{}", coach_b_auth.user.id),
        None::<&()>,
    )
    .await;
    assert_eq!(st_other_user, StatusCode::FORBIDDEN);

    // Mas Player A pode ler o seu próprio perfil cadastral (`GET /api/v1/users/{own_id}`)
    let (st_own_user, own_user): (StatusCode, User) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        &format!("/api/v1/users/{}", player_a_auth.user.id),
        None::<&()>,
    )
    .await;
    assert_eq!(st_own_user, StatusCode::OK);
    assert_eq!(own_user.id, player_a_auth.user.id);

    // 5. Item 3.5: Troca de senha (`POST /api/v1/auth/password`) com rotação imediata da sessão única de 12h
    let new_password = "Padel#2026NewRotatedPass".to_string();
    let (st_pw, pw_res): (StatusCode, ChangePasswordResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::POST,
        "/api/v1/auth/password",
        Some(&ChangePasswordRequest {
            current_password: player_a_pass.clone(),
            new_password: new_password.clone(),
        }),
    )
    .await;
    assert_eq!(st_pw, StatusCode::OK);
    assert_ne!(pw_res.session_token, player_a_auth.session_token);

    // A sessão anterior à troca de senha foi revogada
    let (st_old_sess, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        &player_a_auth.session_token,
        Method::GET,
        "/api/v1/auth/session",
        None::<&()>,
    )
    .await;
    assert_eq!(st_old_sess, StatusCode::UNAUTHORIZED);

    // Login com a senha antiga falha (401) e login com a nova senha funciona (200)
    let (st_old_pw, _): (StatusCode, ErrorResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_a_email.clone(),
            password: player_a_pass.clone(),
        }),
    )
    .await;
    assert_eq!(st_old_pw, StatusCode::UNAUTHORIZED);

    let (st_new_pw, new_login_auth): (StatusCode, AuthResponse) = send_json_with_token(
        &app,
        "",
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_a_email.clone(),
            password: new_password.clone(),
        }),
    )
    .await;
    assert_eq!(st_new_pw, StatusCode::OK);

    // 6. Item 3.6: Defesa Anti-CSRF (`Sec-Fetch-Site: cross-site`) e cabeçalhos `Cache-Control: no-store` + `Content-Security-Policy`
    let csrf_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/password")
        .header(
            "authorization",
            format!("Bearer {}", new_login_auth.session_token),
        )
        .header("sec-fetch-site", "cross-site")
        .body(Body::empty())
        .unwrap();
    let csrf_res = app.clone().oneshot(csrf_req).await.unwrap();
    assert_eq!(csrf_res.status(), StatusCode::FORBIDDEN);

    let sec_headers_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/session")
        .header(
            "authorization",
            format!("Bearer {}", new_login_auth.session_token),
        )
        .body(Body::empty())
        .unwrap();
    let sec_headers_res = app.clone().oneshot(sec_headers_req).await.unwrap();
    assert_eq!(sec_headers_res.status(), StatusCode::OK);
    assert_eq!(
        sec_headers_res
            .headers()
            .get("cache-control")
            .unwrap()
            .to_str()
            .unwrap(),
        "no-store, no-cache, must-revalidate"
    );
    assert!(
        sec_headers_res
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'")
    );
}

#[tokio::test]
async fn test_user_soft_delete_deactivation_and_reactivation() {
    use player_scout::{
        api::auth::{AuthResponse, LoginRequest},
        domain::RegistrationInput,
    };

    let app = setup_test_app().await;
    let unique_uuid = Uuid::now_v7();
    let seed = (unique_uuid.as_u128() & 0xFFFF_FFFF) as u64;
    let player_cpf = generate_unique_valid_cpf(seed + 999);
    let player_email = format!("soft.delete.{unique_uuid}@playerscout.test");
    let password = "StrongPassword2026!#";

    // 1. Cadastra usuário via API
    let reg_req = RegistrationInput {
        first_name: "Usuario".to_string(),
        last_name: "Desativavel".to_string(),
        age: 30,
        cpf: player_cpf.clone(),
        email: player_email.clone(),
        whatsapp_phone: "11988887777".to_string(),
        cep: "01310-100".to_string(),
        street: "Av Paulista".to_string(),
        street_number: "500".to_string(),
        complement: "".to_string(),
        neighborhood: "Bela Vista".to_string(),
        city: "São Paulo".to_string(),
        state: "SP".to_string(),
        role: UserRole::Player,
        preferred_side: PreferredSide::Right,
        dominant_hand: DominantHand::RightHanded,
        password: password.to_string(),
    };

    let (status, reg_auth): (StatusCode, AuthResponse) =
        send_json(&app, Method::POST, "/api/v1/auth/register", Some(&reg_req)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(reg_auth.user.is_active);

    // 2. Faz login para obter token de sessão ativa
    let (status, login_res): (StatusCode, AuthResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_email.clone(),
            password: password.to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let active_token = login_res.session_token;

    // Confirma que a sessão é válida
    let session_check_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/session")
        .header("authorization", format!("Bearer {active_token}"))
        .body(Body::empty())
        .unwrap();
    let session_check_res = app.clone().oneshot(session_check_req).await.unwrap();
    assert_eq!(session_check_res.status(), StatusCode::OK);

    // 3. Admin desativa o usuário (Soft Delete) via DELETE /api/v1/users/{id}
    let admin_token = BOOTSTRAP_SESSION_TOKEN.get().unwrap();
    let deact_req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/users/{}", reg_auth.user.id))
        .header("authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())
        .unwrap();
    let deact_res = app.clone().oneshot(deact_req).await.unwrap();
    assert_eq!(deact_res.status(), StatusCode::OK);
    let deact_bytes = axum::body::to_bytes(deact_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let deactivated_user: User = serde_json::from_slice(&deact_bytes).unwrap();
    assert!(!deactivated_user.is_active);

    // 4. Sessão anterior deve ter sido revogada imediatamente (401 Unauthorized)
    let revoked_check_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/session")
        .header("authorization", format!("Bearer {active_token}"))
        .body(Body::empty())
        .unwrap();
    let revoked_check_res = app.clone().oneshot(revoked_check_req).await.unwrap();
    assert_eq!(revoked_check_res.status(), StatusCode::UNAUTHORIZED);

    // 5. Nova tentativa de login para conta desativada deve ser bloqueada (403 Forbidden)
    let (status, deact_login_err): (StatusCode, ErrorResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_email.clone(),
            password: password.to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(deact_login_err.code, "FORBIDDEN");
    assert!(deact_login_err.error.contains("desativada"));

    // 6. Admin reativa a conta via POST /api/v1/users/{id}/reactivate
    let react_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/users/{}/reactivate", reg_auth.user.id))
        .header("authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())
        .unwrap();
    let react_res = app.clone().oneshot(react_req).await.unwrap();
    assert_eq!(react_res.status(), StatusCode::OK);
    let react_bytes = axum::body::to_bytes(react_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let reactivated_user: User = serde_json::from_slice(&react_bytes).unwrap();
    assert!(reactivated_user.is_active);

    // 7. Login após reativação deve suceder normalmente (200 OK)
    let (status, react_login_ok): (StatusCode, AuthResponse) = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&LoginRequest {
            email: player_email.clone(),
            password: password.to_string(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(react_login_ok.user.id, reg_auth.user.id);
    assert!(react_login_ok.user.is_active);
}
