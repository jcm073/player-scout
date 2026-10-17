use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    InProgress,
    Completed,
    Canceled,
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
            id: Uuid::new_v4(),
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
