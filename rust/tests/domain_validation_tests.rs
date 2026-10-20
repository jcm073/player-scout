use player_scout::domain::*;
use uuid::Uuid;

#[test]
fn test_winner_must_be_hit() {
    let match_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    // Válido: Winner com Hit
    let valid_winner = ShotEvent::new(
        match_id,
        player_id,
        1,
        ShotType::SmashX3,
        ShotOutcome::Hit,
        Some(DecisionQuality::Good),
        Some(SpecialTag::Winner),
    );
    assert!(valid_winner.is_ok());

    // Inválido: Winner com Miss deve ser rejeitado pelo domínio
    let invalid_winner = ShotEvent::new(
        match_id,
        player_id,
        1,
        ShotType::SmashX3,
        ShotOutcome::Miss,
        Some(DecisionQuality::Good),
        Some(SpecialTag::Winner),
    );
    assert_eq!(invalid_winner.unwrap_err(), DomainError::WinnerMustBeHit);
}

#[test]
fn test_unforced_error_must_be_miss() {
    let match_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    // Válido: Erro Não Forçado com Miss
    let valid_enf = ShotEvent::new(
        match_id,
        player_id,
        1,
        ShotType::GloboEsquerda,
        ShotOutcome::Miss,
        Some(DecisionQuality::Good),
        Some(SpecialTag::UnforcedError),
    );
    assert!(valid_enf.is_ok());

    // Inválido: Erro Não Forçado com Hit deve ser rejeitado pelo domínio
    let invalid_enf = ShotEvent::new(
        match_id,
        player_id,
        1,
        ShotType::GloboEsquerda,
        ShotOutcome::Hit,
        Some(DecisionQuality::Good),
        Some(SpecialTag::UnforcedError),
    );
    assert_eq!(
        invalid_enf.unwrap_err(),
        DomainError::UnforcedErrorMustBeMiss
    );
}

#[test]
fn test_default_decision_is_good() {
    let match_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    let shot = ShotEvent::new(
        match_id,
        player_id,
        1,
        ShotType::VoleioDireita,
        ShotOutcome::Hit,
        None, // Omitido pelo treinador
        None,
    )
    .expect("Criação válida de jogada");

    assert_eq!(shot.decision, DecisionQuality::Good);
}

#[test]
fn test_set_number_validation() {
    let match_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    // Sets válidos (1 a 5)
    for set in 1..=5 {
        let shot = ShotEvent::new(
            match_id,
            player_id,
            set,
            ShotType::Saque,
            ShotOutcome::Hit,
            None,
            None,
        );
        assert!(shot.is_ok());
    }

    // Sets inválidos (0 ou 6+)
    let invalid_set_0 = ShotEvent::new(
        match_id,
        player_id,
        0,
        ShotType::Saque,
        ShotOutcome::Hit,
        None,
        None,
    );
    assert_eq!(invalid_set_0.unwrap_err(), DomainError::InvalidSetNumber(0));

    let invalid_set_6 = ShotEvent::new(
        match_id,
        player_id,
        6,
        ShotType::Saque,
        ShotOutcome::Hit,
        None,
        None,
    );
    assert_eq!(invalid_set_6.unwrap_err(), DomainError::InvalidSetNumber(6));
}

#[test]
fn test_user_roles_coach_admin_and_player() {
    // 1. Coach (Treinador): Pode registrar jogadas, não é read-only
    let coach = User::new_coach("Marcos Coach", "marcos@padel.com", "hash123");
    assert_eq!(coach.role, UserRole::Coach);
    assert!(coach.role.is_coach());
    assert!(!coach.role.is_admin());
    assert!(!coach.role.is_player());
    assert!(coach.role.can_write_shots());
    assert!(!coach.role.is_read_only());

    // 2. Admin (Administrador): Controle total
    let admin = User::new_admin("Administrador", "admin@padel.com", "hashadmin");
    assert_eq!(admin.role, UserRole::Admin);
    assert!(admin.role.is_admin());
    assert!(!admin.role.is_coach());
    assert!(!admin.role.is_player());
    assert!(admin.role.can_write_shots());
    assert!(!admin.role.is_read_only());

    // 3. Player (Jogador): Estritamente Read-Only, sem permissão para registrar jogadas
    let player_user = User::new_player("Lucas Silveira", "lucas@padel.com", "hashplayer");
    assert_eq!(player_user.role, UserRole::Player);
    assert!(player_user.role.is_player());
    assert!(!player_user.role.is_coach());
    assert!(!player_user.role.is_admin());
    assert!(player_user.role.is_read_only());
    assert!(!player_user.role.can_write_shots());
}

#[test]
fn test_player_can_be_linked_to_multiple_coaches() {
    let coach1_id = Uuid::new_v4();
    let coach2_id = Uuid::new_v4();
    let player_user_id = Uuid::new_v4();

    // Atleta com conta própria (UserRole::Player)
    let player = Player::new(
        Some(coach1_id),
        "Lucas Silveira",
        DominantHand::RightHanded,
        PreferredSide::Left,
        Some("1ª Categoria Open".to_string()),
    )
    .expect("Criação válida")
    .with_user_account(player_user_id);

    assert_eq!(player.user_id, Some(player_user_id));
    assert_eq!(player.coach_id, Some(coach1_id));

    // Relacionamento N:N: Jogador vinculado ao Coach 1 (Técnico) e Coach 2 (Físico)
    let assoc1 = CoachPlayer::new(coach1_id, player.id);
    let assoc2 = CoachPlayer::new(coach2_id, player.id);

    assert_eq!(assoc1.coach_id, coach1_id);
    assert_eq!(assoc1.player_id, player.id);

    assert_eq!(assoc2.coach_id, coach2_id);
    assert_eq!(assoc2.player_id, player.id);
}

#[test]
fn test_player_without_user_account() {
    let coach_id = Uuid::new_v4();

    // Atleta cadastrado pelo coach sem e-mail/conta própria
    let unregistered_player = Player::new(
        Some(coach_id),
        "Aluno Iniciante",
        DominantHand::RightHanded,
        PreferredSide::Right,
        None,
    )
    .expect("Atleta válido");

    assert_eq!(unregistered_player.user_id, None);
    assert_eq!(unregistered_player.coach_id, Some(coach_id));

    // Validação de nome não vazio
    let invalid_player = Player::new(
        Some(coach_id),
        "   ",
        DominantHand::RightHanded,
        PreferredSide::Left,
        None,
    );
    assert_eq!(invalid_player.unwrap_err(), DomainError::EmptyPlayerName);
}

#[test]
fn test_21_padel_shots_categorization() {
    // Início de Ponto (2)
    assert!(ShotType::Saque.is_serve_or_return());
    assert!(ShotType::Recepcao.is_serve_or_return());

    // Rede & Ataque (10)
    let attack_shots = [
        ShotType::VoleioDireita,
        ShotType::VoleioEsquerda,
        ShotType::Bandeja,
        ShotType::Vibora,
        ShotType::SmashPotencia,
        ShotType::SmashX3,
        ShotType::SmashX4,
        ShotType::Rulo,
        ShotType::Deixadinha,
        ShotType::Bloqueio,
    ];
    for shot in attack_shots {
        assert!(shot.is_net_or_attack());
        assert!(!shot.is_defense_or_backcourt());
    }

    // Fundo & Vidros (9)
    let defense_shots = [
        ShotType::BatidaDireita,
        ShotType::BatidaEsquerda,
        ShotType::GloboDireita,
        ShotType::GloboEsquerda,
        ShotType::Chiquita,
        ShotType::SaidaParedeDireita,
        ShotType::SaidaParedeEsquerda,
        ShotType::BajadaParede,
        ShotType::ContraParede,
    ];
    for shot in defense_shots {
        assert!(shot.is_defense_or_backcourt());
        assert!(!shot.is_net_or_attack());
    }
}

#[test]
fn test_empty_analytics_report_has_zero_safe_values() {
    let player_id = Uuid::now_v7();
    let report = PlayerAnalyticsReport::from_events(player_id, AnalyticsFilter::default(), &[]);

    assert_eq!(report.total_shots, 0);
    assert_eq!(report.hit_rate_pct, 0.0);
    assert_eq!(report.good_decision_rate_pct, 0.0);
    assert_eq!(report.winner_to_ue_balance, 0);
    assert_eq!(report.matrix.hit_good, 0);
    assert_eq!(report.radar.tactical_maturity_pct, 0.0);
    assert!(report.by_shot_type.is_empty());
}

#[test]
fn test_matrix_and_radar_analytics_calculation() {
    let match_id = Uuid::now_v7();
    let player_id = Uuid::now_v7();

    let events = vec![
        // 1. Hit + Good (Jogada Ideal) - Saque
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::Saque,
            ShotOutcome::Hit,
            Some(DecisionQuality::Good),
            None,
        )
        .unwrap(),
        // 2. Hit + Bad (Compensação/Sorte) - VoleioDireita + Winner
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::VoleioDireita,
            ShotOutcome::Hit,
            Some(DecisionQuality::Bad),
            Some(SpecialTag::Winner),
        )
        .unwrap(),
        // 3. Miss + Good (Erro Técnico) - Bandeja + UnforcedError
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::Bandeja,
            ShotOutcome::Miss,
            Some(DecisionQuality::Good),
            Some(SpecialTag::UnforcedError),
        )
        .unwrap(),
        // 4. Miss + Bad (Erro Tático Grave) - BajadaParede + UnforcedError
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::BajadaParede,
            ShotOutcome::Miss,
            Some(DecisionQuality::Bad),
            Some(SpecialTag::UnforcedError),
        )
        .unwrap(),
    ];

    let report = PlayerAnalyticsReport::from_events(player_id, AnalyticsFilter::default(), &events);

    assert_eq!(report.total_shots, 4);
    assert_eq!(report.hits, 2);
    assert_eq!(report.misses, 2);
    assert_eq!(report.hit_rate_pct, 50.0);
    assert_eq!(report.good_decisions, 2);
    assert_eq!(report.bad_decisions, 2);
    assert_eq!(report.good_decision_rate_pct, 50.0);
    assert_eq!(report.winners, 1);
    assert_eq!(report.unforced_errors, 2);
    assert_eq!(report.winner_to_ue_balance, -1);

    // 4 quadrantes da Matriz Decisão × Execução (1 em cada quadrante = 25% cada)
    assert_eq!(report.matrix.hit_good, 1);
    assert_eq!(report.matrix.hit_good_pct, 25.0);
    assert_eq!(report.matrix.hit_bad, 1);
    assert_eq!(report.matrix.hit_bad_pct, 25.0);
    assert_eq!(report.matrix.miss_good, 1);
    assert_eq!(report.matrix.miss_good_pct, 25.0);
    assert_eq!(report.matrix.miss_bad, 1);
    assert_eq!(report.matrix.miss_bad_pct, 25.0);

    // Zonas & Radar
    assert_eq!(report.zones.serve_return.hit_rate_pct, 100.0);
    assert_eq!(report.zones.net_attack.hit_rate_pct, 50.0);
    assert_eq!(report.zones.backcourt_glass.hit_rate_pct, 0.0);
    assert_eq!(report.radar.serve_consistency_pct, 100.0);
    assert_eq!(report.radar.net_control_pct, 100.0);
    assert_eq!(report.radar.overhead_power_pct, 0.0);
    assert_eq!(report.radar.wall_play_pct, 0.0);
    assert_eq!(report.radar.tactical_maturity_pct, 50.0);
}

#[test]
fn test_analytics_comparison_deltas_set1_vs_set2() {
    let match_id = Uuid::now_v7();
    let player_id = Uuid::now_v7();

    let events = vec![
        // Set 1: 1 acerto e 1 erro (50% hit rate, 50% good decision, 0 W, 1 UE => saldo -1)
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::Vibora,
            ShotOutcome::Hit,
            Some(DecisionQuality::Good),
            None,
        )
        .unwrap(),
        ShotEvent::new(
            match_id,
            player_id,
            1,
            ShotType::Vibora,
            ShotOutcome::Miss,
            Some(DecisionQuality::Bad),
            Some(SpecialTag::UnforcedError),
        )
        .unwrap(),
        // Set 2: 2 acertos e 0 erros (100% hit rate, 100% good decision, 1 W, 0 UE => saldo +1)
        ShotEvent::new(
            match_id,
            player_id,
            2,
            ShotType::Vibora,
            ShotOutcome::Hit,
            Some(DecisionQuality::Good),
            Some(SpecialTag::Winner),
        )
        .unwrap(),
        ShotEvent::new(
            match_id,
            player_id,
            2,
            ShotType::SmashX3,
            ShotOutcome::Hit,
            Some(DecisionQuality::Good),
            None,
        )
        .unwrap(),
    ];

    let scope_a = AnalyticsFilter {
        match_id: Some(match_id),
        set_number: Some(1),
        ..Default::default()
    };
    let scope_b = AnalyticsFilter {
        match_id: Some(match_id),
        set_number: Some(2),
        ..Default::default()
    };

    let report_a = PlayerAnalyticsReport::from_events(player_id, scope_a, &events);
    let report_b = PlayerAnalyticsReport::from_events(player_id, scope_b, &events);
    let comparison = AnalyticsComparisonReport::compare(report_a, report_b);

    assert_eq!(comparison.deltas.hit_rate_delta_pct, 50.0); // 100% - 50% = +50%
    assert_eq!(comparison.deltas.good_decision_rate_delta_pct, 50.0); // 100% - 50% = +50%
    assert_eq!(comparison.deltas.winner_to_ue_balance_delta, 2); // +1 - (-1) = +2
    assert_eq!(comparison.deltas.winners_delta, 1);
    assert_eq!(comparison.deltas.unforced_errors_delta, -1);
}

#[test]
fn test_argon2i_password_hashing_and_policy_validation() {
    use player_scout::domain::{hash_password_argon2i, validate_password, verify_password_argon2i};

    // Menos de 12 caracteres -> inválido
    assert!(validate_password("Padel2026!").is_err());
    // Sem letra maiúscula -> inválido
    assert!(validate_password("padelpro2026!@").is_err());
    // Sem letra minúscula -> inválido
    assert!(validate_password("PADELPRO2026!@").is_err());
    // Sem número -> inválido
    assert!(validate_password("PadelProMaster!@").is_err());
    // Sem caractere especial -> inválido
    assert!(validate_password("PadelPro2026Ab").is_err());

    // Senha válida: 14 caracteres, maiúscula, minúscula, número e especial
    let valid_pass = "PadelPro2026!@";
    assert!(validate_password(valid_pass).is_ok());

    let hash = hash_password_argon2i(valid_pass).expect("Deve gerar hash Argon2i");
    assert!(
        hash.starts_with("$argon2i$v=19$"),
        "O hash deve usar explicitamente o algoritmo Argon2i: {hash}"
    );
    assert!(verify_password_argon2i(valid_pass, &hash));
    assert!(!verify_password_argon2i("SenhaErrada2026!@", &hash));
}

#[test]
fn test_cpf_modulo_11_and_complement_validation() {
    use player_scout::domain::{
        validate_and_normalize_cep, validate_and_normalize_cpf, validate_complement,
    };

    // CPF válido com máscara -> normaliza para 11 dígitos
    let cpf = validate_and_normalize_cpf("529.982.247-25").expect("CPF válido");
    assert_eq!(cpf, "52998224725");

    // CPF com dígitos repetidos -> inválido
    assert!(validate_and_normalize_cpf("111.111.111-11").is_err());
    // CPF com dígito verificador incorreto -> inválido
    assert!(validate_and_normalize_cpf("529.982.247-20").is_err());

    // CEP válido com máscara -> normaliza para 8 dígitos
    let cep = validate_and_normalize_cep("90010-150").expect("CEP válido");
    assert_eq!(cep, "90010150");
    assert!(validate_and_normalize_cep("9001-150").is_err());

    // Complemento: vazio permitido (casas sem bloco/apto)
    assert_eq!(validate_complement("   ").unwrap(), "");
    // Complemento válido com até 10 caracteres alfanuméricos sem especiais
    assert_eq!(validate_complement("Apto 302B").unwrap(), "Apto 302B");
    // Complemento > 10 caracteres -> inválido
    assert!(validate_complement("Apartamento 302").is_err());
    // Complemento com caractere especial -> inválido
    assert!(validate_complement("Apto #302").is_err());
    assert!(validate_complement("Bloco-A").is_err());
}

#[test]
fn test_session_token_generation_and_argon2i_verification() {
    use player_scout::domain::{
        ACCOUNT_LOCKOUT_SECONDS, MAX_FAILED_LOGIN_ATTEMPTS, SESSION_TTL_HOURS, SESSION_TTL_SECONDS,
        extract_bearer_token, extract_session_token_from_cookie,
        generate_and_hash_session_token_argon2i, parse_session_token_id,
        verify_session_token_argon2i,
    };

    // Verifica constantes de segurança (12 horas = 43.200s e 5 tentativas = 15 min)
    assert_eq!(SESSION_TTL_HOURS, 12);
    assert_eq!(SESSION_TTL_SECONDS, 43_200);
    assert_eq!(MAX_FAILED_LOGIN_ATTEMPTS, 5);
    assert_eq!(ACCOUNT_LOCKOUT_SECONDS, 900);

    let session_id = Uuid::now_v7();
    let (raw_token, token_hash) =
        generate_and_hash_session_token_argon2i(session_id).expect("deve gerar token e hash");

    assert!(raw_token.starts_with("ps_sess_"));
    assert!(
        token_hash.starts_with("$argon2i$v=19$"),
        "O hash do token de sessão deve utilizar a mesma biblioteca Argon2i: {token_hash}"
    );

    let parsed_id = parse_session_token_id(&raw_token).expect("deve extrair session_id");
    assert_eq!(parsed_id, session_id);

    assert!(verify_session_token_argon2i(&raw_token, &token_hash));
    assert!(!verify_session_token_argon2i(
        "ps_sess_00000000000000000000000000000000_segredoinvalido123456",
        &token_hash
    ));

    let cookie_header = format!("theme=dark; ps_session={raw_token}; SameSite=Lax");
    assert_eq!(
        extract_session_token_from_cookie(&cookie_header).as_deref(),
        Some(raw_token.as_str())
    );
    let bearer_header = format!("Bearer {raw_token}");
    assert_eq!(
        extract_bearer_token(&bearer_header).as_deref(),
        Some(raw_token.as_str())
    );
}

#[test]
fn test_registration_blocks_admin_privilege_escalation_and_bola_actor_resolution() {
    use chrono::Utc;
    use player_scout::domain::{
        AuthenticatedUser, DominantHand, PreferredSide, RegistrationInput, User, UserSession,
        verify_dummy_password_argon2i,
    };

    // 1. Auto-cadastro com role Admin deve ser bloqueado na camada de domínio
    let admin_reg = RegistrationInput {
        first_name: "Invasor".to_string(),
        last_name: "Malicioso".to_string(),
        age: 30,
        cpf: "529.982.247-25".to_string(),
        email: "invasor@padel.com".to_string(),
        whatsapp_phone: "11988887777".to_string(),
        cep: "01310-100".to_string(),
        street: "Av Paulista".to_string(),
        street_number: "1000".to_string(),
        complement: "Sala 1".to_string(),
        neighborhood: "Bela Vista".to_string(),
        city: "São Paulo".to_string(),
        state: "SP".to_string(),
        role: UserRole::Admin,
        preferred_side: PreferredSide::Left,
        dominant_hand: DominantHand::RightHanded,
        password: "Padel#2026StrongPass".to_string(),
        oauth_provider: None,
        oauth_provider_user_id: None,
    };

    let err = admin_reg
        .validate_and_hash()
        .expect_err("não deve permitir auto-cadastro como Admin");
    assert_eq!(err, DomainError::ForbiddenRoleInRegistration);
    assert_eq!(err.code(), "FORBIDDEN_ROLE_IN_REGISTRATION");

    // 2. Proteção Anti-Spoofing (BOLA): Player/Coach não podem falsificar actor_id de terceiros
    let now = Utc::now();
    let player_user = User::new_player("Atleta A", "a@padel.com", "hash");
    assert!(player_user.is_active);
    let other_user_id = Uuid::now_v7();
    let session = UserSession {
        id: Uuid::now_v7(),
        user_id: player_user.id,
        created_at_epoch: now.timestamp(),
        expires_at_epoch: now.timestamp() + 43_200,
        created_at: now,
        expires_at: now,
        ttl_seconds: 43_200,
    };
    let auth_player = AuthenticatedUser {
        user: player_user.clone(),
        session,
    };

    // Sem actor_id explícito -> resolve para o próprio usuário autenticado
    assert_eq!(auth_player.resolve_actor_id(None).unwrap(), player_user.id);
    // Com o próprio actor_id -> permitido
    assert_eq!(
        auth_player.resolve_actor_id(Some(player_user.id)).unwrap(),
        player_user.id
    );
    // Com actor_id de outro usuário -> bloqueado (403 Forbidden)
    assert!(auth_player.resolve_actor_id(Some(other_user_id)).is_err());
    // require_coach_or_admin -> bloqueado para Player
    assert!(auth_player.require_coach_or_admin().is_err());

    // 3. Executa dummy Argon2i para garantir que não entra em pânico
    verify_dummy_password_argon2i("QualquerSenhaTentada123!");
}

#[tokio::test]
async fn test_oauth2_providers_urls_and_profile_validation() {
    // 1. Validação de URLs e especificações dos provedores (Google, Microsoft, Instagram)
    let google = OAuthProvider::Google;
    assert_eq!(google.as_str(), "google");
    assert_eq!(google.display_name(), "Google (Gmail)");
    assert_eq!(google.token_url(), "https://oauth2.googleapis.com/token");
    assert_eq!(
        google.userinfo_url(),
        "https://openidconnect.googleapis.com/v1/userinfo"
    );

    let auth_url = google.authorize_url("client123", "http://localhost:3000/callback", "state789");
    assert!(auth_url.contains("client_id=client123"));
    assert!(auth_url.contains("state=state789"));
    assert!(auth_url.contains("accounts.google.com/o/oauth2/v2/auth"));

    let ms = OAuthProvider::Microsoft;
    assert_eq!(ms.as_str(), "microsoft");
    assert_eq!(ms.display_name(), "Microsoft Office");

    let ig = OAuthProvider::Instagram;
    assert_eq!(ig.as_str(), "instagram");
    assert_eq!(ig.display_name(), "Instagram");

    // 2. Validação de troca de código Google sem credenciais -> deve retornar erro interno claro
    let http_client = reqwest::Client::new();
    let exchange_err =
        exchange_google_authorization_code(&http_client, "", "", "http://localhost", "fake_code")
            .await;
    assert!(exchange_err.is_err());
    let err_str = exchange_err.unwrap_err().to_string();
    assert!(err_str.contains("Credenciais Google OAuth2 não configuradas no ambiente"));

    // 3. Validação de RegistrationInput com campos de auto-vinculação OAuth2
    let reg_input = RegistrationInput {
        first_name: "Carlos".to_string(),
        last_name: "Alcaraz".to_string(),
        age: 21,
        cpf: "12345678909".to_string(),
        email: "carlos@google.com".to_string(),
        whatsapp_phone: "11999998888".to_string(),
        cep: "01310-100".to_string(),
        street: "Av Paulista".to_string(),
        street_number: "1000".to_string(),
        complement: "".to_string(),
        neighborhood: "Bela Vista".to_string(),
        city: "São Paulo".to_string(),
        state: "SP".to_string(),
        role: UserRole::Player,
        preferred_side: PreferredSide::Right,
        dominant_hand: DominantHand::RightHanded,
        password: "ValidPassword123#".to_string(),
        oauth_provider: Some("google".to_string()),
        oauth_provider_user_id: Some("google-sub-123456".to_string()),
    };

    let validated = reg_input.validate_and_hash();
    assert!(validated.is_ok());
    let val = validated.unwrap();
    assert_eq!(val.full_name, "Carlos Alcaraz");
    assert_eq!(val.email, "carlos@google.com");
}
