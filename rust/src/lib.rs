pub mod api;
pub mod config;
pub mod domain;
pub mod error;

pub use config::AppConfig;
pub use error::{AppError, Result};

/// Cria a aplicação Axum com todas as rotas configuradas
pub fn create_app() -> axum::Router {
    api::app_router()
}
