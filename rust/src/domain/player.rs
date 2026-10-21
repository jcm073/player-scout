use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DomainError;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    toasty::Embed,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("dominant_hand_enum"))]
pub enum DominantHand {
    #[default]
    RightHanded,
    LeftHanded,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    toasty::Embed,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("preferred_side_enum"))]
pub enum PreferredSide {
    #[default]
    Left,
    Right,
    Both,
}

impl PreferredSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreferredSide::Left => "left",
            PreferredSide::Right => "right",
            PreferredSide::Both => "both",
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            PreferredSide::Left => "Esquerda (Revés)",
            PreferredSide::Right => "Direita (Drive)",
            PreferredSide::Both => "Ambos os Lados",
        }
    }
}

/// Perfil do Atleta no Player Scout
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Player {
    pub id: Uuid,
    /// Vínculo opcional com a conta de usuário (UserRole::Player) para acesso read-only ao dashboard
    pub user_id: Option<Uuid>,
    /// Treinador principal de referência (opcional, relação formal N:N via CoachPlayer)
    pub coach_id: Option<Uuid>,
    pub name: String,
    pub dominant_hand: DominantHand,
    pub preferred_side: PreferredSide,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Player {
    pub fn new(
        coach_id: Option<Uuid>,
        name: impl Into<String>,
        dominant_hand: DominantHand,
        preferred_side: PreferredSide,
        category: Option<String>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyPlayerName);
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::now_v7(),
            user_id: None,
            coach_id,
            name,
            dominant_hand,
            preferred_side,
            category,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_user_account(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

/// Associação N:N entre Treinador e Jogador
/// Permite que um mesmo jogador treine com múltiplos treinadores (ex: técnico, físico e tático)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CoachPlayer {
    pub coach_id: Uuid,
    pub player_id: Uuid,
    pub assigned_at: DateTime<Utc>,
}

impl CoachPlayer {
    pub fn new(coach_id: Uuid, player_id: Uuid) -> Self {
        Self {
            coach_id,
            player_id,
            assigned_at: Utc::now(),
        }
    }
}
