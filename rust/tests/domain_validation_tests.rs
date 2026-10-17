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
