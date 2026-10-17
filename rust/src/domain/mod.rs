pub mod analytics;
pub mod match_session;
pub mod player;
pub mod shot;
pub mod user;

pub use analytics::*;
pub use match_session::*;
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
}

impl DomainError {
    pub fn code(&self) -> &'static str {
        match self {
            DomainError::WinnerMustBeHit => "WINNER_MUST_BE_HIT",
            DomainError::UnforcedErrorMustBeMiss => "UNFORCED_ERROR_MUST_BE_MISS",
            DomainError::InvalidSetNumber(_) => "INVALID_SET_NUMBER",
            DomainError::EmptyPlayerName => "EMPTY_PLAYER_NAME",
        }
    }
}
