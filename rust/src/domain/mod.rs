pub mod analytics;
pub mod auth;
pub mod match_session;
pub mod oauth;
pub mod player;
pub mod shot;
pub mod user;

pub use analytics::*;
pub use auth::*;
pub use match_session::*;
pub use oauth::*;
pub use player::*;
pub use shot::*;
pub use user::*;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error(
        "Uma jogada marcada como Winner é um ponto finalizado com sucesso e deve ter resultado 'acertou' (hit)"
    )]
    WinnerMustBeHit,

    #[error("Um Erro Não Forçado deve obrigatoriamente ter resultado 'errou' (miss)")]
    UnforcedErrorMustBeMiss,

    #[error("Número do set inválido: {0}. Um jogo possui entre 1 e 5 sets.")]
    InvalidSetNumber(u8),

    #[error("O nome do jogador não pode ser vazio")]
    EmptyPlayerName,

    #[error("Senha inválida: {0}")]
    InvalidPassword(String),

    #[error("CPF inválido: {0}")]
    InvalidCpf(String),

    #[error("E-mail inválido: {0}")]
    InvalidEmail(String),

    #[error("Telefone de WhatsApp inválido: {0}")]
    InvalidWhatsappPhone(String),

    #[error("CEP inválido: {0}")]
    InvalidCep(String),

    #[error("Complemento inválido: {0}")]
    InvalidComplement(String),

    #[error("Idade inválida: {0}. A idade deve estar entre 5 e 120 anos.")]
    InvalidAge(u8),

    #[error("Estado (UF) inválido: {0}")]
    InvalidStateUf(String),

    #[error("Campo obrigatório não preenchido: {0}")]
    MissingRequiredField(&'static str),

    #[error(
        "Escalação de privilégio bloqueada: o perfil 'admin' não pode ser criado por auto-cadastro público (permitido apenas 'player' ou 'coach')"
    )]
    ForbiddenRoleInRegistration,
}

impl DomainError {
    pub fn code(&self) -> &'static str {
        match self {
            DomainError::WinnerMustBeHit => "WINNER_MUST_BE_HIT",
            DomainError::UnforcedErrorMustBeMiss => "UNFORCED_ERROR_MUST_BE_MISS",
            DomainError::InvalidSetNumber(_) => "INVALID_SET_NUMBER",
            DomainError::EmptyPlayerName => "EMPTY_PLAYER_NAME",
            DomainError::InvalidPassword(_) => "INVALID_PASSWORD",
            DomainError::InvalidCpf(_) => "INVALID_CPF",
            DomainError::InvalidEmail(_) => "INVALID_EMAIL",
            DomainError::InvalidWhatsappPhone(_) => "INVALID_WHATSAPP_PHONE",
            DomainError::InvalidCep(_) => "INVALID_CEP",
            DomainError::InvalidComplement(_) => "INVALID_COMPLEMENT",
            DomainError::InvalidAge(_) => "INVALID_AGE",
            DomainError::InvalidStateUf(_) => "INVALID_STATE_UF",
            DomainError::MissingRequiredField(_) => "MISSING_REQUIRED_FIELD",
            DomainError::ForbiddenRoleInRegistration => "FORBIDDEN_ROLE_IN_REGISTRATION",
        }
    }
}
