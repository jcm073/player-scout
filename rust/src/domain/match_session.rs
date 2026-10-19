use crate::domain::player::PreferredSide;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("match_status_enum"))]
pub enum MatchStatus {
    InProgress,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchTeam {
    TeamA,
    TeamB,
}

impl MatchTeam {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TeamA => "team_a",
            Self::TeamB => "team_b",
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            Self::TeamA => "Dupla A",
            Self::TeamB => "Dupla B",
        }
    }
}

impl std::str::FromStr for MatchTeam {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "team_a" | "TeamA" | "A" => Ok(Self::TeamA),
            "team_b" | "TeamB" | "B" => Ok(Self::TeamB),
            other => Err(format!("Time inválido: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPlayer {
    pub match_id: Uuid,
    pub player_id: Uuid,
    pub team: MatchTeam,
    pub court_side: PreferredSide,
    pub is_scouted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSession {
    pub id: Uuid,
    pub coach_id: Uuid,
    pub player_id: Uuid,
    pub tournament_id: Option<Uuid>,
    pub opponent_name: Option<String>,
    pub match_date: DateTime<Utc>,
    pub status: MatchStatus,
    pub score: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MatchSession {
    pub fn new(coach_id: Uuid, player_id: Uuid, opponent_name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            coach_id,
            player_id,
            tournament_id: None,
            opponent_name,
            match_date: now,
            status: MatchStatus::InProgress,
            score: None,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }
}
