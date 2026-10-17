use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Coach,
    Player,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Coach => "coach",
            UserRole::Player => "player",
        }
    }

    pub fn is_coach(&self) -> bool {
        matches!(self, UserRole::Coach)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn is_player(&self) -> bool {
        matches!(self, UserRole::Player)
    }

    /// O perfil de jogador é estritamente de leitura (somente consulta ao seu próprio histórico)
    pub fn is_read_only(&self) -> bool {
        matches!(self, UserRole::Player)
    }

    /// Permissão para registro e alteração de jogadas em quadra (apenas Coach e Admin)
    pub fn can_write_shots(&self) -> bool {
        matches!(self, UserRole::Coach | UserRole::Admin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub role: UserRole,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new_coach(
        name: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            role: UserRole::Coach,
            name: name.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_admin(
        name: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            role: UserRole::Admin,
            name: name.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_player(
        name: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            role: UserRole::Player,
            name: name.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
