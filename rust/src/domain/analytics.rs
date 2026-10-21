use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DecisionQuality, ShotEvent, ShotOutcome, ShotType, SpecialTag};

/// Escopo recursivo de filtragem para agregação analítica:
/// Permite consultar desde a carreira inteira do atleta até um ano, mês, partida ou set específico.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsFilter {
    pub match_id: Option<Uuid>,
    pub set_number: Option<u8>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

impl AnalyticsFilter {
    pub fn matches_event(&self, event: &ShotEvent) -> bool {
        if let Some(m_id) = self.match_id
            && event.match_id != m_id
        {
            return false;
        }
        if let Some(set_num) = self.set_number
            && event.set_number != set_num
        {
            return false;
        }
        if let Some(from) = self.from_date
            && event.created_at < from
        {
            return false;
        }
        if let Some(to) = self.to_date
            && event.created_at > to
        {
            return false;
        }
        true
    }
}

/// Matriz Tática de Ouro (Decisão × Execução em 4 quadrantes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DecisionExecutionMatrix {
    /// Acertou + Decisão Boa (Jogada Ideal)
    pub hit_good: usize,
    pub hit_good_pct: f64,
    /// Acertou + Decisão Ruim (Compensação / Sorte)
    pub hit_bad: usize,
    pub hit_bad_pct: f64,
    /// Errou + Decisão Boa (Erro puramente Técnico)
    pub miss_good: usize,
    pub miss_good_pct: f64,
    /// Errou + Decisão Ruim (Erro Tático Grave)
    pub miss_bad: usize,
    pub miss_bad_pct: f64,
}

/// Estatística agregada de uma zona da quadra
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ZoneStat {
    pub total: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate_pct: f64,
    pub good_decision_rate_pct: f64,
}

/// Eficiência dividida pelas 3 grandes zonas do Padel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ZoneMetrics {
    pub serve_return: ZoneStat,
    pub net_attack: ZoneStat,
    pub backcourt_glass: ZoneStat,
}

/// Radar de Habilidades nas 6 fases fundamentais do Padel (0.0 a 100.0)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PadelRadarMetrics {
    /// 1. Consistência de Saque & Recepção
    pub serve_consistency_pct: f64,
    /// 2. Domínio de Rede (Voleios, Bloqueio, Deixadinha)
    pub net_control_pct: f64,
    /// 3. Jogo Aéreo & Definição (Bandeja, Víbora, Smashes, Rulo)
    pub overhead_power_pct: f64,
    /// 4. Fundo de Quadra & Globos (Batidas, Globos, Chiquita)
    pub backcourt_defense_pct: f64,
    /// 5. Uso dos Vidros / Paredes (Saídas, Bajada, Contra-parede)
    pub wall_play_pct: f64,
    /// 6. Maturidade Tática (% de Decisões Boas)
    pub tactical_maturity_pct: f64,
}

/// Desempenho detalhado por tipo de golpe do Padel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ShotTypeBreakdown {
    pub shot_type: ShotType,
    pub total: usize,
    pub hits: usize,
    pub misses: usize,
    pub winners: usize,
    pub unforced_errors: usize,
    pub hit_rate_pct: f64,
    pub good_decision_rate_pct: f64,
}

/// Relatório Analítico Recursivo de um Atleta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlayerAnalyticsReport {
    pub player_id: Uuid,
    pub scope: AnalyticsFilter,
    pub total_shots: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate_pct: f64,
    pub miss_rate_pct: f64,
    pub good_decisions: usize,
    pub bad_decisions: usize,
    pub good_decision_rate_pct: f64,
    pub bad_decision_rate_pct: f64,
    pub winners: usize,
    pub unforced_errors: usize,
    pub winner_to_ue_balance: isize,
    pub matrix: DecisionExecutionMatrix,
    pub zones: ZoneMetrics,
    pub radar: PadelRadarMetrics,
    pub by_shot_type: Vec<ShotTypeBreakdown>,
}

/// Evolução por tipo de golpe entre dois escopos (B - A)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ShotTypeComparisonDelta {
    pub shot_type: ShotType,
    pub total_a: usize,
    pub total_b: usize,
    pub hit_rate_a_pct: f64,
    pub hit_rate_b_pct: f64,
    pub hit_rate_delta_pct: f64,
    pub good_decision_delta_pct: f64,
}

/// Deltas consolidados do Modo Comparativo Lado a Lado (Escopo B - Escopo A)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ComparisonDeltas {
    /// Variação na taxa de acerto (B - A) em pontos percentuais
    pub hit_rate_delta_pct: f64,
    /// Variação na qualidade de decisão tática (B - A) em pontos percentuais
    pub good_decision_rate_delta_pct: f64,
    /// Variação no saldo Winner/ENF (B - A)
    pub winner_to_ue_balance_delta: isize,
    /// Variação na contagem absoluta de Winners (B - A)
    pub winners_delta: isize,
    /// Variação na contagem absoluta de Erros Não Forçados (B - A)
    pub unforced_errors_delta: isize,
    /// Comparativo pareado por tipo de jogada
    pub by_shot_type_deltas: Vec<ShotTypeComparisonDelta>,
}

/// Relatório Comparativo Lado a Lado (Escopo A vs Escopo B)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsComparisonReport {
    pub player_id: Uuid,
    pub scope_a: PlayerAnalyticsReport,
    pub scope_b: PlayerAnalyticsReport,
    pub deltas: ComparisonDeltas,
}

fn round_pct(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let raw = (numerator as f64 / denominator as f64) * 100.0;
    (raw * 100.0).round() / 100.0
}

fn round_two_decimals(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

fn compute_zone_stat<'a>(events: impl Iterator<Item = &'a ShotEvent>) -> ZoneStat {
    let mut total = 0usize;
    let mut hits = 0usize;
    let mut good = 0usize;

    for ev in events {
        total += 1;
        if ev.outcome == ShotOutcome::Hit {
            hits += 1;
        }
        if ev.decision == DecisionQuality::Good {
            good += 1;
        }
    }

    let misses = total.saturating_sub(hits);
    ZoneStat {
        total,
        hits,
        misses,
        hit_rate_pct: round_pct(hits, total),
        good_decision_rate_pct: round_pct(good, total),
    }
}

fn compute_hit_rate_for<'a>(events: impl Iterator<Item = &'a ShotEvent>) -> f64 {
    let mut total = 0usize;
    let mut hits = 0usize;
    for ev in events {
        total += 1;
        if ev.outcome == ShotOutcome::Hit {
            hits += 1;
        }
    }
    round_pct(hits, total)
}

impl PlayerAnalyticsReport {
    /// Constrói o relatório analítico completo a partir de uma lista de eventos de jogada
    pub fn from_events(player_id: Uuid, scope: AnalyticsFilter, events: &[ShotEvent]) -> Self {
        let filtered: Vec<&ShotEvent> = events
            .iter()
            .filter(|ev| ev.player_id == player_id && scope.matches_event(ev))
            .collect();

        let total_shots = filtered.len();
        let mut hits = 0usize;
        let mut good_decisions = 0usize;
        let mut winners = 0usize;
        let mut unforced_errors = 0usize;

        let mut hit_good = 0usize;
        let mut hit_bad = 0usize;
        let mut miss_good = 0usize;
        let mut miss_bad = 0usize;

        // Agrupamento ordenado por nome serializado do golpe
        let mut by_type_map: BTreeMap<String, (ShotType, Vec<&ShotEvent>)> = BTreeMap::new();

        for ev in &filtered {
            let is_hit = ev.outcome == ShotOutcome::Hit;
            let is_good = ev.decision == DecisionQuality::Good;

            if is_hit {
                hits += 1;
            }
            if is_good {
                good_decisions += 1;
            }

            match (is_hit, is_good) {
                (true, true) => hit_good += 1,
                (true, false) => hit_bad += 1,
                (false, true) => miss_good += 1,
                (false, false) => miss_bad += 1,
            }

            match ev.special_tag {
                Some(SpecialTag::Winner) => winners += 1,
                Some(SpecialTag::UnforcedError) => unforced_errors += 1,
                None => {}
            }

            let key = serde_json::to_string(&ev.shot_type).unwrap_or_default();
            by_type_map
                .entry(key)
                .or_insert_with(|| (ev.shot_type, Vec::new()))
                .1
                .push(*ev);
        }

        let misses = total_shots.saturating_sub(hits);
        let bad_decisions = total_shots.saturating_sub(good_decisions);

        let matrix = DecisionExecutionMatrix {
            hit_good,
            hit_good_pct: round_pct(hit_good, total_shots),
            hit_bad,
            hit_bad_pct: round_pct(hit_bad, total_shots),
            miss_good,
            miss_good_pct: round_pct(miss_good, total_shots),
            miss_bad,
            miss_bad_pct: round_pct(miss_bad, total_shots),
        };

        let serve_return = compute_zone_stat(
            filtered
                .iter()
                .copied()
                .filter(|e| e.shot_type.is_serve_or_return()),
        );
        let net_attack = compute_zone_stat(
            filtered
                .iter()
                .copied()
                .filter(|e| e.shot_type.is_net_or_attack()),
        );
        let backcourt_glass = compute_zone_stat(
            filtered
                .iter()
                .copied()
                .filter(|e| e.shot_type.is_defense_or_backcourt()),
        );

        let zones = ZoneMetrics {
            serve_return: serve_return.clone(),
            net_attack,
            backcourt_glass,
        };

        // Radar de 6 fases do Padel
        let net_control_pct = compute_hit_rate_for(filtered.iter().copied().filter(|e| {
            matches!(
                e.shot_type,
                ShotType::VoleioDireita
                    | ShotType::VoleioEsquerda
                    | ShotType::Bloqueio
                    | ShotType::Deixadinha
            )
        }));

        let overhead_power_pct = compute_hit_rate_for(filtered.iter().copied().filter(|e| {
            matches!(
                e.shot_type,
                ShotType::Bandeja
                    | ShotType::Vibora
                    | ShotType::SmashPotencia
                    | ShotType::SmashX3
                    | ShotType::SmashX4
                    | ShotType::Rulo
            )
        }));

        let backcourt_defense_pct = compute_hit_rate_for(filtered.iter().copied().filter(|e| {
            matches!(
                e.shot_type,
                ShotType::BatidaDireita
                    | ShotType::BatidaEsquerda
                    | ShotType::GloboDireita
                    | ShotType::GloboEsquerda
                    | ShotType::Chiquita
            )
        }));

        let wall_play_pct = compute_hit_rate_for(filtered.iter().copied().filter(|e| {
            matches!(
                e.shot_type,
                ShotType::SaidaParedeDireita
                    | ShotType::SaidaParedeEsquerda
                    | ShotType::BajadaParede
                    | ShotType::ContraParede
            )
        }));

        let good_decision_rate_pct = round_pct(good_decisions, total_shots);

        let radar = PadelRadarMetrics {
            serve_consistency_pct: serve_return.hit_rate_pct,
            net_control_pct,
            overhead_power_pct,
            backcourt_defense_pct,
            wall_play_pct,
            tactical_maturity_pct: good_decision_rate_pct,
        };

        let mut by_shot_type = Vec::with_capacity(by_type_map.len());
        for (_key, (shot_type, group)) in by_type_map {
            let total = group.len();
            let s_hits = group
                .iter()
                .filter(|e| e.outcome == ShotOutcome::Hit)
                .count();
            let s_misses = total.saturating_sub(s_hits);
            let s_good = group
                .iter()
                .filter(|e| e.decision == DecisionQuality::Good)
                .count();
            let s_winners = group
                .iter()
                .filter(|e| e.special_tag == Some(SpecialTag::Winner))
                .count();
            let s_ues = group
                .iter()
                .filter(|e| e.special_tag == Some(SpecialTag::UnforcedError))
                .count();

            by_shot_type.push(ShotTypeBreakdown {
                shot_type,
                total,
                hits: s_hits,
                misses: s_misses,
                winners: s_winners,
                unforced_errors: s_ues,
                hit_rate_pct: round_pct(s_hits, total),
                good_decision_rate_pct: round_pct(s_good, total),
            });
        }

        Self {
            player_id,
            scope,
            total_shots,
            hits,
            misses,
            hit_rate_pct: round_pct(hits, total_shots),
            miss_rate_pct: round_pct(misses, total_shots),
            good_decisions,
            bad_decisions,
            good_decision_rate_pct,
            bad_decision_rate_pct: round_pct(bad_decisions, total_shots),
            winners,
            unforced_errors,
            winner_to_ue_balance: (winners as isize) - (unforced_errors as isize),
            matrix,
            zones,
            radar,
            by_shot_type,
        }
    }
}

impl AnalyticsComparisonReport {
    /// Compara dois relatórios analíticos (Escopo A vs Escopo B) e calcula os deltas (B - A)
    pub fn compare(scope_a: PlayerAnalyticsReport, scope_b: PlayerAnalyticsReport) -> Self {
        let player_id = scope_a.player_id;

        let hit_rate_delta_pct = round_two_decimals(scope_b.hit_rate_pct - scope_a.hit_rate_pct);
        let good_decision_rate_delta_pct =
            round_two_decimals(scope_b.good_decision_rate_pct - scope_a.good_decision_rate_pct);
        let winner_to_ue_balance_delta =
            scope_b.winner_to_ue_balance - scope_a.winner_to_ue_balance;
        let winners_delta = (scope_b.winners as isize) - (scope_a.winners as isize);
        let unforced_errors_delta =
            (scope_b.unforced_errors as isize) - (scope_a.unforced_errors as isize);

        // Unifica todos os tipos de golpe presentes em A ou B
        let mut types_map: BTreeMap<
            String,
            (
                ShotType,
                Option<&ShotTypeBreakdown>,
                Option<&ShotTypeBreakdown>,
            ),
        > = BTreeMap::new();

        for item_a in &scope_a.by_shot_type {
            let key = serde_json::to_string(&item_a.shot_type).unwrap_or_default();
            types_map.insert(key, (item_a.shot_type, Some(item_a), None));
        }

        for item_b in &scope_b.by_shot_type {
            let key = serde_json::to_string(&item_b.shot_type).unwrap_or_default();
            types_map
                .entry(key)
                .and_modify(|entry| entry.2 = Some(item_b))
                .or_insert((item_b.shot_type, None, Some(item_b)));
        }

        let mut by_shot_type_deltas = Vec::with_capacity(types_map.len());
        for (_k, (shot_type, opt_a, opt_b)) in types_map {
            let total_a = opt_a.map_or(0, |x| x.total);
            let total_b = opt_b.map_or(0, |x| x.total);
            let hit_rate_a_pct = opt_a.map_or(0.0, |x| x.hit_rate_pct);
            let hit_rate_b_pct = opt_b.map_or(0.0, |x| x.hit_rate_pct);
            let good_a_pct = opt_a.map_or(0.0, |x| x.good_decision_rate_pct);
            let good_b_pct = opt_b.map_or(0.0, |x| x.good_decision_rate_pct);

            by_shot_type_deltas.push(ShotTypeComparisonDelta {
                shot_type,
                total_a,
                total_b,
                hit_rate_a_pct,
                hit_rate_b_pct,
                hit_rate_delta_pct: round_two_decimals(hit_rate_b_pct - hit_rate_a_pct),
                good_decision_delta_pct: round_two_decimals(good_b_pct - good_a_pct),
            });
        }

        let deltas = ComparisonDeltas {
            hit_rate_delta_pct,
            good_decision_rate_delta_pct,
            winner_to_ue_balance_delta,
            winners_delta,
            unforced_errors_delta,
            by_shot_type_deltas,
        };

        Self {
            player_id,
            scope_a,
            scope_b,
            deltas,
        }
    }
}
