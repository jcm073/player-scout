use player_scout::{AppConfig, Database, create_app};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.rust_log.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🎾 Inicializando Player Scout Backend (Axum + Toasty ORM + PostgreSQL 18)");

    let db = Database::connect(&config.database_url).await?;
    let app = create_app(db);
    let addr = config.server_addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 Servidor REST rodando em http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
