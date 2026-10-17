use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DomainError;

/// Catálogo Oficial com as 21 jogadas mapeadas no Padel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    SmashX3,
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
}

/// Resultado obrigatório da execução do golpe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotOutcome {
    Hit,
    Miss,
}

/// Qualidade da decisão tática do jogador (Padrão: Good)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionQuality {
    #[default]
    Good,
    Bad,
}

/// Marcador especial opcional da jogada
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
            id: Uuid::new_v4(),
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
