use player_scout::{AppConfig, create_app};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match AppConfig::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "Aviso: Falha ao carregar configuração completa ({err}). Usando padrões de desenvolvimento."
            );
            AppConfig {
                database_url: "postgresql://postgres:scout123@localhost:5432/playerscout_dev"
                    .to_string(),
                server_host: "0.0.0.0".to_string(),
                server_port: 3000,
                rust_log: "info,player_scout=debug".to_string(),
            }
        }
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.rust_log.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🎾 Inicializando Player Scout Backend (Axum + PostgreSQL 18)");

    let app = create_app();
    let addr = config.server_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 Servidor REST rodando em http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
