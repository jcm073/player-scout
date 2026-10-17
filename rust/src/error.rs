use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Erro de validação de domínio: {0}")]
    Domain(#[from] crate::domain::DomainError),

    #[error("Recurso não encontrado: {0}")]
    NotFound(String),

    #[error("Acesso negado: {0}")]
    Forbidden(String),

    #[error("Não autenticado: {0}")]
    Unauthorized(String),

    #[error("Muitas tentativas de login: {0}")]
    TooManyRequests(String),

    #[error("Conflito de recurso: {0}")]
    Conflict(String),

    #[error("Requisição inválida: {0}")]
    BadRequest(String),

    #[error("Erro no banco de dados: {0}")]
    Database(String),

    #[error("Erro interno no servidor: {0}")]
    Internal(String),
}

impl From<toasty::Error> for AppError {
    fn from(err: toasty::Error) -> Self {
        if err.is_record_not_found() {
            AppError::NotFound(err.to_string())
        } else {
            AppError::Database(err.to_string())
        }
    }
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::Internal(format!("Erro ao renderizar template Askama: {err}"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Domain(err) => (StatusCode::UNPROCESSABLE_ENTITY, err.code()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, "TOO_MANY_REQUESTS"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        });

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
