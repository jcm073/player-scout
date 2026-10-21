use player_scout::{
    AppConfig, Database,
    db::NewShotInput,
    domain::{
        DecisionQuality, DominantHand, MatchTeam, PreferredSide, RegistrationInput, ShotOutcome,
        ShotType, SpecialTag, UserRole,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());
    let config = AppConfig::from_env().expect("DATABASE_URL deve estar configurada no .env");

    println!("🎾 Conectando ao PostgreSQL 18 via Toasty ORM...");
    let db = Database::connect(&config.database_url).await?;

    println!("👤 Criando/verificando Treinador de Demonstração...");
    let coach_email = "treinador@padel.com";
    let coach_password = "PadelScout#2026";

    let coach = match db.find_user_by_key(Some(coach_email), None).await {
        Ok(existing) => {
            println!(
                "   ℹ️ Treinador já existe: {} ({})",
                existing.name, existing.email
            );
            existing
        }
        Err(_) => {
            let registration = RegistrationInput {
                first_name: "Carlos".to_string(),
                last_name: "Treinador Demo".to_string(),
                age: 38,
                cpf: "529.982.247-25".to_string(),
                email: coach_email.to_string(),
                whatsapp_phone: "+5511999998888".to_string(),
                cep: "01310-100".to_string(),
                street: "Avenida Paulista".to_string(),
                street_number: "1000".to_string(),
                complement: "Sala 42".to_string(),
                neighborhood: "Bela Vista".to_string(),
                city: "São Paulo".to_string(),
                state: "SP".to_string(),
                role: UserRole::Coach,
                preferred_side: PreferredSide::Right,
                dominant_hand: DominantHand::RightHanded,
                password: coach_password.to_string(),
                oauth_provider: None,
                oauth_provider_user_id: None,
            };
            let (new_coach, _) = db.register_user_with_profile(registration).await?;
            println!(
                "   ✅ Treinador criado: {} ({})",
                new_coach.name, new_coach.email
            );
            new_coach
        }
    };

    println!("\n👥 Criando os 4 Atletas de Demonstração...");
    let athletes_data = [
        (
            "Lucas 'Bandeja' Silva",
            DominantHand::RightHanded,
            PreferredSide::Left,
            "1ª Categoria",
        ),
        (
            "Gabriel 'Víbora' Santos",
            DominantHand::RightHanded,
            PreferredSide::Right,
            "1ª Categoria",
        ),
        (
            "Matheus 'Paredes' Oliveira",
            DominantHand::LeftHanded,
            PreferredSide::Right,
            "2ª Categoria",
        ),
        (
            "Felipe 'Chiquita' Costa",
            DominantHand::RightHanded,
            PreferredSide::Left,
            "2ª Categoria",
        ),
    ];

    let mut players = Vec::new();
    let existing_players = db
        .list_players_for_actor(coach.id)
        .await
        .unwrap_or_default();

    for (name, hand, side, cat) in athletes_data {
        if let Some(existing) = existing_players.iter().find(|p| p.name == name) {
            println!(
                "   ℹ️ Atleta já existe: {} ({:?})",
                existing.name, existing.preferred_side
            );
            players.push(existing.clone());
        } else {
            let p = db
                .create_player(
                    coach.id,
                    None,
                    name.to_string(),
                    hand,
                    side,
                    Some(cat.to_string()),
                )
                .await?;
            println!(
                "   ✅ Atleta criado: {} ({})",
                p.name,
                p.preferred_side.label_pt()
            );
            players.push(p);
        }
    }

    if players.len() < 4 {
        eprintln!("Erro: Menos de 4 atletas disponíveis para a partida de duplas");
        return Ok(());
    }

    let p1 = &players[0]; // Dupla A - Revés (Left)
    let p2 = &players[1]; // Dupla A - Drive (Right)
    let p3 = &players[2]; // Dupla B - Drive (Right)
    let p4 = &players[3]; // Dupla B - Revés (Left)

    println!("\n🏆 Criando Partida Finalizada com 50 Jogadas (Sets 1 e 2)...");
    let match_finished = db
        .create_match(
            coach.id,
            p1.id,
            Some("Dupla Rival Pro".to_string()),
            Some("Grande final do torneio regional de Padel".to_string()),
        )
        .await?;

    // Escalar os 4 atletas
    let _ = db
        .assign_match_player(
            match_finished.id,
            p1.id,
            MatchTeam::TeamA,
            PreferredSide::Left,
            true,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_finished.id,
            p2.id,
            MatchTeam::TeamA,
            PreferredSide::Right,
            true,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_finished.id,
            p3.id,
            MatchTeam::TeamB,
            PreferredSide::Right,
            false,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_finished.id,
            p4.id,
            MatchTeam::TeamB,
            PreferredSide::Left,
            false,
        )
        .await;

    // Injetar jogadas para o Atleta Principal (Lucas) no Set 1 e Set 2
    let set1_shots = [
        (
            ShotType::Saque,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Vibora,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::VoleioDireita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::VoleioEsquerda,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Chiquita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::SmashPotencia,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::GloboDireita,
            ShotOutcome::Miss,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::BatidaDireita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::SaidaParedeDireita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::BajadaParede,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::Deixadinha,
            ShotOutcome::Miss,
            DecisionQuality::Bad,
            Some(SpecialTag::UnforcedError),
        ),
        (
            ShotType::Bloqueio,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
    ];

    for (shot_type, outcome, decision, tag) in set1_shots {
        db.record_shot(
            coach.id,
            match_finished.id,
            NewShotInput {
                player_id: p1.id,
                set_number: 1,
                shot_type,
                outcome,
                decision: Some(decision),
                special_tag: tag,
            },
        )
        .await?;
    }

    let set2_shots = [
        (
            ShotType::Saque,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Miss,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Vibora,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::SmashX3,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::VoleioDireita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::VoleioEsquerda,
            ShotOutcome::Miss,
            DecisionQuality::Bad,
            Some(SpecialTag::UnforcedError),
        ),
        (
            ShotType::Chiquita,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::SaidaParedeEsquerda,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::ContraParede,
            ShotOutcome::Hit,
            DecisionQuality::Bad,
            None,
        ),
        (
            ShotType::Rulo,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
        (
            ShotType::BatidaEsquerda,
            ShotOutcome::Miss,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::GloboEsquerda,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::Bandeja,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            None,
        ),
        (
            ShotType::SmashPotencia,
            ShotOutcome::Hit,
            DecisionQuality::Good,
            Some(SpecialTag::Winner),
        ),
    ];

    for (shot_type, outcome, decision, tag) in set2_shots {
        db.record_shot(
            coach.id,
            match_finished.id,
            NewShotInput {
                player_id: p1.id,
                set_number: 2,
                shot_type,
                outcome,
                decision: Some(decision),
                special_tag: tag,
            },
        )
        .await?;
    }

    // Finalizar a partida
    db.finish_match(coach.id, match_finished.id, Some("6-4, 7-5".to_string()))
        .await?;
    println!(
        "   ✅ Partida finalizada com 30 jogadas computadas para {}",
        p1.name
    );

    println!("\n🎾 Criando Partida Em Andamento para Teste de Quadra Touch-First...");
    let match_active = db
        .create_match(
            coach.id,
            p1.id,
            Some("Dupla Adversária Treino".to_string()),
            Some("Treino tático de duplas em quadra".to_string()),
        )
        .await?;

    let _ = db
        .assign_match_player(
            match_active.id,
            p1.id,
            MatchTeam::TeamA,
            PreferredSide::Left,
            true,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_active.id,
            p2.id,
            MatchTeam::TeamA,
            PreferredSide::Right,
            true,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_active.id,
            p3.id,
            MatchTeam::TeamB,
            PreferredSide::Right,
            true,
        )
        .await;
    let _ = db
        .assign_match_player(
            match_active.id,
            p4.id,
            MatchTeam::TeamB,
            PreferredSide::Left,
            true,
        )
        .await;

    println!(
        "   ✅ Partida ativa criada com 4 atletas na quadra (ID: {})",
        match_active.id
    );

    println!("\n=======================================================");
    println!("🎉 SEED EXECUTADO COM SUCESSO!");
    println!("=======================================================");
    println!("Treinador Cadastrado: {}", coach_email);
    println!("Senha Padrão:         {}", coach_password);
    println!("Atletas no Banco:     {} atletas prontos", players.len());
    println!("Partidas Criadas:     1 Finalizada (6-4, 7-5) + 1 Ativa em Quadra");
    println!("Swagger UI:           http://localhost:3000/swagger-ui/");
    println!("Dashboard Web:        http://localhost:3000/web/login");
    println!("=======================================================\n");

    Ok(())
}
