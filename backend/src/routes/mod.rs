//! HTTP surface. No auth in this pass (spec § 10 keeps accounts out of v1 signup).

pub mod characters;
pub mod content;

use crate::AppState;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/characters", post(characters::create_character))
        .route(
            "/characters/:id",
            get(characters::get_character).patch(characters::patch_character),
        )
        .route("/characters/:id/roll", post(characters::roll))
        .route("/characters/:id/finalize", post(characters::finalize))
        .route("/content/careers", get(content::list_careers))
        .route("/content/hobbies", get(content::list_hobbies))
        .route("/content/skills", get(content::list_skills))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
