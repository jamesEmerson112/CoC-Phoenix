//! Server entry point: connect, migrate, load the content pack, serve.

use coc_phoenix::config::Config;
use coc_phoenix::{db, routes, AppState};
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let config = Config::from_env();
    tracing::info!(bind = %config.bind, "starting coc-phoenix backend");

    let pool = db::connect(&config.database_url, config.max_connections)
        .await
        .map_err(|e| {
            format!(
                "could not reach Postgres at DATABASE_URL ({e}). \
                 Start one with `docker compose up -d` from backend/."
            )
        })?;

    if config.run_migrations {
        db::migrate(&pool).await.map_err(|e| e.to_string())?;
        tracing::info!("migrations applied");
    }

    // Content is data, published after the schema and outside the checksummed migration chain,
    // so editing `content/starter_pack.json` reaches existing databases instead of bricking
    // their boot. Idempotent: safe on every start.
    if config.sync_content {
        db::sync_content_pack(&pool, &coc_phoenix::content::starter_pack_file())
            .await
            .map_err(|e| e.to_string())?;
        tracing::info!("bundled content pack published");
    }

    let catalog = db::load_catalog(&pool, &config.content_pack_id)
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(
        pack = %catalog.pack_id,
        careers = catalog.careers().len(),
        hobbies = catalog.hobbies().len(),
        skills = catalog.skills().len(),
        "content pack loaded"
    );

    let app = routes::router(AppState::new(pool, catalog));
    let listener = tokio::net::TcpListener::bind(&config.bind)
        .await
        .map_err(|e| format!("could not bind {}: {e}", config.bind))?;
    tracing::info!("listening on http://{}", config.bind);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| e.to_string())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
