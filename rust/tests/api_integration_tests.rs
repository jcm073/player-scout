use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use player_scout::{
    AppConfig, Database,
    api::{
        matches::CreateMatchRequest,
        players::{AssignCoachRequest, CreatePlayerRequest},
        shots::{RecordShotRequest, UndoShotRequest},
        users::CreateUserRequest,
    },
    create_app,
    domain::{
        CoachPlayer, DecisionQuality, DominantHand, MatchSession, Player, PreferredSide, ShotEvent,
        ShotOutcome, ShotType, SpecialTag, User, UserRole,
    },
    error::ErrorResponse,
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> axum::Router {
    // Carrega .env da raiz do workspace ou do diretório atual
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());
    let config = AppConfig::from_env().expect("DATABASE_URL deve estar configurada no .env");
    let db = Database::connect(&config.database_url)
        .await
        .expect("Falha ao conectar ao PostgreSQL 18 via Toasty ORM");
    create_app(db)
}

async fn send_json<T: serde::Serialize, R: DeserializeOwned>(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<&T>,
) -> (StatusCode, R) {
    let payload = match body {
        Some(b) => serde_json::to_vec(b).expect("Falha ao serializar JSON"),
        None => Vec::new(),
    };

    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
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
}
