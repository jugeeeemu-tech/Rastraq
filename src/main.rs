use anyhow::Result;
use rastraq::{app::build_router, db::Database};
use std::{env, net::SocketAddr};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "rastraq=info,tower_http=info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("RASTRAQ_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/rastraq.sqlite?mode=rwc".to_string());
    if database_url.starts_with("sqlite://data/") {
        std::fs::create_dir_all("data")?;
    }
    let db = Database::connect(&database_url).await?;
    db.migrate().await?;

    let app = build_router(db)
        .nest_service("/", ServeDir::new("web/dist").append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http());
    let addr: SocketAddr = env::var("RASTRAQ_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()?;
    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
