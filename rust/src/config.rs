use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub rust_log: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            AppError::Internal(
                "A variável DATABASE_URL deve ser definida no ambiente ou .env".to_string(),
            )
        })?;

        let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| {
                AppError::Internal(
                    "A variável SERVER_PORT deve ser um número de porta válido (ex: 3000)"
                        .to_string(),
                )
            })?;

        let rust_log =
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,player_scout=debug".to_string());

        Ok(Self {
            database_url,
            server_host,
            server_port,
            rust_log,
        })
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
