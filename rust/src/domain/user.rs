use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PreferredSide;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, toasty::Embed)]
#[serde(rename_all = "snake_case")]
#[column(type = enum("user_role_enum"))]
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
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub age: Option<u8>,
    pub cpf: Option<String>,
    pub email: String,
    pub whatsapp_phone: Option<String>,
    pub cep: Option<String>,
    pub street: Option<String>,
    pub street_number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub preferred_side: PreferredSide,
    #[serde(skip_serializing, default)]
    pub password_hash: String,
    #[serde(default = "default_active")]
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_active() -> bool {
    true
}

impl User {
    pub fn new_coach(
        name: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            role: UserRole::Coach,
            name: name.into(),
            first_name: None,
            last_name: None,
            age: None,
            cpf: None,
            email: email.into(),
            whatsapp_phone: None,
            cep: None,
            street: None,
            street_number: None,
            complement: None,
            neighborhood: None,
            city: None,
            state: None,
            preferred_side: PreferredSide::Left,
            password_hash: password_hash.into(),
            is_active: true,
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
            id: Uuid::now_v7(),
            role: UserRole::Admin,
            name: name.into(),
            first_name: None,
            last_name: None,
            age: None,
            cpf: None,
            email: email.into(),
            whatsapp_phone: None,
            cep: None,
            street: None,
            street_number: None,
            complement: None,
            neighborhood: None,
            city: None,
            state: None,
            preferred_side: PreferredSide::Left,
            password_hash: password_hash.into(),
            is_active: true,
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
            id: Uuid::now_v7(),
            role: UserRole::Player,
            name: name.into(),
            first_name: None,
            last_name: None,
            age: None,
            cpf: None,
            email: email.into(),
            whatsapp_phone: None,
            cep: None,
            street: None,
            street_number: None,
            complement: None,
            neighborhood: None,
            city: None,
            state: None,
            preferred_side: PreferredSide::Left,
            password_hash: password_hash.into(),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}
