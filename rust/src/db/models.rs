use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    CoachPlayer, DecisionQuality, DominantHand, MatchSession, MatchStatus, OAuthProvider, Player,
    PreferredSide, ShotEvent, ShotOutcome, ShotType, SpecialTag, User, UserOAuthAccount, UserRole,
    UserSession,
};

/// Extrai o timestamp exato embutido em um UUIDv7 ou retorna `Utc::now()` para UUIDv4
pub fn timestamp_from_uuid(id: &Uuid) -> DateTime<Utc> {
    if let Some(ts) = id.get_timestamp() {
        let (secs, nanos) = ts.to_unix();
        if let Some(dt) = DateTime::from_timestamp(secs as i64, nanos) {
            return dt;
        }
    }
    Utc::now()
}

/// Modelo Toasty mapeado para a tabela `users` do PostgreSQL 18
#[derive(Debug, Clone, toasty::Model)]
#[table = "users"]
pub struct UserRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    pub role: UserRole,
    pub name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub age: Option<u8>,
    #[unique]
    pub cpf: Option<String>,
    #[unique]
    pub email: String,
    pub whatsapp_phone: Option<String>,
    pub cep: Option<String>,
    pub street: Option<String>,
    pub street_number: Option<String>,
    pub complement: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub preferred_side: PreferredSide,
    pub password_hash: String,
    pub failed_login_attempts: u8,
    pub locked_until_epoch: Option<i64>,
}

impl From<UserRecord> for User {
    fn from(rec: UserRecord) -> Self {
        let ts = timestamp_from_uuid(&rec.id);
        Self {
            id: rec.id,
            role: rec.role,
            name: rec.name,
            first_name: rec.first_name,
            last_name: rec.last_name,
            age: rec.age,
            cpf: rec.cpf,
            email: rec.email,
            whatsapp_phone: rec.whatsapp_phone,
            cep: rec.cep,
            street: rec.street,
            street_number: rec.street_number,
            complement: rec.complement,
            neighborhood: rec.neighborhood,
            city: rec.city,
            state: rec.state,
            preferred_side: rec.preferred_side,
            password_hash: rec.password_hash,
            created_at: ts,
            updated_at: ts,
        }
    }
}

/// Modelo Toasty mapeado para a tabela `user_sessions` do PostgreSQL 18:
/// - `#[unique] user_id` garante sessão única por usuário
/// - `token_hash` armazena o hash Argon2i do token opaco
#[derive(Debug, Clone, toasty::Model)]
#[table = "user_sessions"]
pub struct UserSessionRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    #[unique]
    pub user_id: Uuid,
    pub token_hash: String,
    pub created_at_epoch: i64,
    pub expires_at_epoch: i64,
}

impl From<UserSessionRecord> for UserSession {
    fn from(rec: UserSessionRecord) -> Self {
        let created_at = DateTime::from_timestamp(rec.created_at_epoch, 0).unwrap_or_else(Utc::now);
        let expires_at = DateTime::from_timestamp(rec.expires_at_epoch, 0).unwrap_or_else(Utc::now);
        let now_epoch = Utc::now().timestamp();
        Self {
            id: rec.id,
            user_id: rec.user_id,
            created_at_epoch: rec.created_at_epoch,
            expires_at_epoch: rec.expires_at_epoch,
            created_at,
            expires_at,
            ttl_seconds: (rec.expires_at_epoch - now_epoch).max(0),
        }
    }
}

/// Modelo Toasty mapeado para a tabela `user_oauth_accounts` do PostgreSQL 18
#[derive(Debug, Clone, toasty::Model)]
#[table = "user_oauth_accounts"]
pub struct UserOAuthRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    #[index]
    pub user_id: Uuid,
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub provider_email: String,
}

impl From<UserOAuthRecord> for UserOAuthAccount {
    fn from(rec: UserOAuthRecord) -> Self {
        let ts = timestamp_from_uuid(&rec.id);
        Self {
            id: rec.id,
            user_id: rec.user_id,
            provider: rec.provider,
            provider_user_id: rec.provider_user_id,
            provider_email: rec.provider_email,
            created_at: ts,
        }
    }
}

/// Modelo Toasty mapeado para a tabela `players` do PostgreSQL 18
#[derive(Debug, Clone, toasty::Model)]
#[table = "players"]
pub struct PlayerRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    #[unique]
    pub user_id: Option<Uuid>,
    #[index]
    pub coach_id: Option<Uuid>,
    pub name: String,
    pub dominant_hand: DominantHand,
    pub preferred_side: PreferredSide,
    pub category: Option<String>,
}

impl From<PlayerRecord> for Player {
    fn from(rec: PlayerRecord) -> Self {
        let ts = timestamp_from_uuid(&rec.id);
        Self {
            id: rec.id,
            user_id: rec.user_id,
            coach_id: rec.coach_id,
            name: rec.name,
            dominant_hand: rec.dominant_hand,
            preferred_side: rec.preferred_side,
            category: rec.category,
            created_at: ts,
            updated_at: ts,
        }
    }
}

/// Modelo Toasty mapeado para a tabela associativa `coach_players` (N:N)
#[derive(Debug, Clone, toasty::Model)]
#[table = "coach_players"]
#[key(partition = coach_id, local = player_id)]
pub struct CoachPlayerRecord {
    pub coach_id: Uuid,
    pub player_id: Uuid,
}

impl From<CoachPlayerRecord> for CoachPlayer {
    fn from(rec: CoachPlayerRecord) -> Self {
        Self {
            coach_id: rec.coach_id,
            player_id: rec.player_id,
            assigned_at: Utc::now(),
        }
    }
}

/// Modelo Toasty mapeado para a tabela `matches` do PostgreSQL 18
#[derive(Debug, Clone, toasty::Model)]
#[table = "matches"]
pub struct MatchRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    #[index]
    pub coach_id: Uuid,
    #[index]
    pub player_id: Uuid,
    pub tournament_id: Option<Uuid>,
    pub opponent_name: Option<String>,
    pub status: MatchStatus,
    pub score: Option<String>,
    pub notes: Option<String>,
}

impl From<MatchRecord> for MatchSession {
    fn from(rec: MatchRecord) -> Self {
        let ts = timestamp_from_uuid(&rec.id);
        Self {
            id: rec.id,
            coach_id: rec.coach_id,
            player_id: rec.player_id,
            tournament_id: rec.tournament_id,
            opponent_name: rec.opponent_name,
            match_date: ts,
            status: rec.status,
            score: rec.score,
            notes: rec.notes,
            created_at: ts,
            updated_at: ts,
        }
    }
}

/// Modelo Toasty mapeado para a tabela `shot_events` do PostgreSQL 18
#[derive(Debug, Clone, toasty::Model)]
#[table = "shot_events"]
pub struct ShotEventRecord {
    #[key]
    #[auto]
    pub id: Uuid,
    #[index]
    pub match_id: Uuid,
    #[index]
    pub player_id: Uuid,
    pub set_number: u8,
    pub shot_type: ShotType,
    pub outcome: ShotOutcome,
    pub decision: DecisionQuality,
    pub special_tag: Option<SpecialTag>,
}

impl From<ShotEventRecord> for ShotEvent {
    fn from(rec: ShotEventRecord) -> Self {
        let ts = timestamp_from_uuid(&rec.id);
        Self {
            id: rec.id,
            match_id: rec.match_id,
            player_id: rec.player_id,
            set_number: rec.set_number,
            shot_type: rec.shot_type,
            outcome: rec.outcome,
            decision: rec.decision,
            special_tag: rec.special_tag,
            created_at: ts,
        }
    }
}
