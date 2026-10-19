use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DomainError;

/// Catálogo Oficial com as 21 jogadas mapeadas no Padel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("shot_type_enum"))]
pub enum ShotType {
    // Início de Ponto
    Saque,
    Recepcao,

    // Rede & Ataque
    VoleioDireita,
    VoleioEsquerda,
    Bandeja,
    Vibora,
    SmashPotencia,
    #[column(variant = "smash_x3")]
    SmashX3,
    #[column(variant = "smash_x4")]
    SmashX4,
    Rulo,
    Deixadinha,
    Bloqueio,

    // Fundo & Vidros
    BatidaDireita,
    BatidaEsquerda,
    GloboDireita,
    GloboEsquerda,
    Chiquita,
    SaidaParedeDireita,
    SaidaParedeEsquerda,
    BajadaParede,
    ContraParede,
}

impl ShotType {
    pub fn is_net_or_attack(&self) -> bool {
        matches!(
            self,
            ShotType::VoleioDireita
                | ShotType::VoleioEsquerda
                | ShotType::Bandeja
                | ShotType::Vibora
                | ShotType::SmashPotencia
                | ShotType::SmashX3
                | ShotType::SmashX4
                | ShotType::Rulo
                | ShotType::Deixadinha
                | ShotType::Bloqueio
        )
    }

    pub fn is_defense_or_backcourt(&self) -> bool {
        matches!(
            self,
            ShotType::BatidaDireita
                | ShotType::BatidaEsquerda
                | ShotType::GloboDireita
                | ShotType::GloboEsquerda
                | ShotType::Chiquita
                | ShotType::SaidaParedeDireita
                | ShotType::SaidaParedeEsquerda
                | ShotType::BajadaParede
                | ShotType::ContraParede
        )
    }

    pub fn is_serve_or_return(&self) -> bool {
        matches!(self, ShotType::Saque | ShotType::Recepcao)
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            ShotType::Saque => "Saque",
            ShotType::Recepcao => "Recepção",
            ShotType::VoleioDireita => "Voleio Direita",
            ShotType::VoleioEsquerda => "Voleio Esquerda",
            ShotType::Bandeja => "Bandeja",
            ShotType::Vibora => "Víbora",
            ShotType::SmashPotencia => "Smash",
            ShotType::SmashX3 => "Smash x3",
            ShotType::SmashX4 => "Smash x4",
            ShotType::Rulo => "Rulo na Grade",
            ShotType::Deixadinha => "Deixadinha",
            ShotType::Bloqueio => "Bloqueio",
            ShotType::BatidaDireita => "Batida Direita",
            ShotType::BatidaEsquerda => "Batida Esquerda",
            ShotType::GloboDireita => "Globo Direita",
            ShotType::GloboEsquerda => "Globo Esquerda",
            ShotType::Chiquita => "Chiquita",
            ShotType::SaidaParedeDireita => "Saída Parede Dir.",
            ShotType::SaidaParedeEsquerda => "Saída Parede Esq.",
            ShotType::BajadaParede => "Bajada de Parede",
            ShotType::ContraParede => "Contra-parede",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            ShotType::Saque => "saque",
            ShotType::Recepcao => "recepcao",
            ShotType::VoleioDireita => "voleio_direita",
            ShotType::VoleioEsquerda => "voleio_esquerda",
            ShotType::Bandeja => "bandeja",
            ShotType::Vibora => "vibora",
            ShotType::SmashPotencia => "smash_potencia",
            ShotType::SmashX3 => "smash_x3",
            ShotType::SmashX4 => "smash_x4",
            ShotType::Rulo => "rulo",
            ShotType::Deixadinha => "deixadinha",
            ShotType::Bloqueio => "bloqueio",
            ShotType::BatidaDireita => "batida_direita",
            ShotType::BatidaEsquerda => "batida_esquerda",
            ShotType::GloboDireita => "globo_direita",
            ShotType::GloboEsquerda => "globo_esquerda",
            ShotType::Chiquita => "chiquita",
            ShotType::SaidaParedeDireita => "saida_parede_direita",
            ShotType::SaidaParedeEsquerda => "saida_parede_esquerda",
            ShotType::BajadaParede => "bajada_parede",
            ShotType::ContraParede => "contra_parede",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            ShotType::Saque => "#06b6d4",
            ShotType::Recepcao => "#14b8a6",
            ShotType::VoleioDireita => "#6366f1",
            ShotType::VoleioEsquerda => "#8b5cf6",
            ShotType::Bandeja => "#eab308",
            ShotType::Vibora => "#f59e0b",
            ShotType::SmashPotencia => "#ec4899",
            ShotType::SmashX3 => "#d946ef",
            ShotType::SmashX4 => "#a855f7",
            ShotType::Rulo => "#84cc16",
            ShotType::Deixadinha => "#f43f5e",
            ShotType::Bloqueio => "#64748b",
            ShotType::BatidaDireita => "#22c55e",
            ShotType::BatidaEsquerda => "#10b981",
            ShotType::GloboDireita => "#38bdf8",
            ShotType::GloboEsquerda => "#0284c7",
            ShotType::Chiquita => "#a3e635",
            ShotType::SaidaParedeDireita => "#b45309",
            ShotType::SaidaParedeEsquerda => "#92400e",
            ShotType::BajadaParede => "#e11d48",
            ShotType::ContraParede => "#94a3b8",
        }
    }

    pub const fn serve_shots() -> &'static [ShotType] {
        &[ShotType::Saque, ShotType::Recepcao]
    }

    pub const fn net_shots() -> &'static [ShotType] {
        &[
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
        ]
    }

    pub const fn backcourt_shots() -> &'static [ShotType] {
        &[
            ShotType::BatidaDireita,
            ShotType::BatidaEsquerda,
            ShotType::GloboDireita,
            ShotType::GloboEsquerda,
            ShotType::Chiquita,
            ShotType::SaidaParedeDireita,
            ShotType::SaidaParedeEsquerda,
            ShotType::BajadaParede,
            ShotType::ContraParede,
        ]
    }
}

/// Resultado obrigatório da execução do golpe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("shot_outcome_enum"))]
pub enum ShotOutcome {
    Hit,
    Miss,
}

/// Qualidade da decisão tática do jogador (Padrão: Good)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("decision_quality_enum"))]
pub enum DecisionQuality {
    #[default]
    Good,
    Bad,
}

/// Marcador especial opcional da jogada
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("special_tag_enum"))]
pub enum SpecialTag {
    Winner,
    UnforcedError,
}

/// Evento atômico de uma jogada registrada em quadra
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotEvent {
    pub id: Uuid,
    pub match_id: Uuid,
    pub player_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: DecisionQuality,
    pub special_tag: Option<SpecialTag>,
    pub created_at: DateTime<Utc>,
}

impl ShotEvent {
    pub fn new(
        match_id: Uuid,
        player_id: Uuid,
        set_number: u8,
        shot_type: ShotType,
        outcome: ShotOutcome,
        decision: Option<DecisionQuality>,
        special_tag: Option<SpecialTag>,
    ) -> Result<Self, DomainError> {
        let event = Self {
            id: Uuid::now_v7(),
            match_id,
            player_id,
            set_number,
            shot_type,
            outcome,
            decision: decision.unwrap_or_default(),
            special_tag,
            created_at: Utc::now(),
        };

        event.validate()?;
        Ok(event)
    }

    /// Validações estritas das regras de negócio do esporte
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.set_number < 1 || self.set_number > 5 {
            return Err(DomainError::InvalidSetNumber(self.set_number));
        }

        if let Some(SpecialTag::Winner) = self.special_tag
            && self.outcome != ShotOutcome::Hit
        {
            return Err(DomainError::WinnerMustBeHit);
        }

        if let Some(SpecialTag::UnforcedError) = self.special_tag
            && self.outcome != ShotOutcome::Miss
        {
            return Err(DomainError::UnforcedErrorMustBeMiss);
        }

        Ok(())
    }
}
