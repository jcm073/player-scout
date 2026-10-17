pub mod api;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;

pub use config::AppConfig;
pub use db::Database;
pub use error::{AppError, Result};

/// Cria a aplicação Axum com todas as rotas e o pool Toasty configurados
pub fn create_app(db: Database) -> axum::Router {
    api::app_router(db)
}
