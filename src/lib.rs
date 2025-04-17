//! lib was created just for benches

mod api;
mod app_state;
mod kahan;
// mod monotonic_queue;
mod error;
mod shared_monotonic_queue;
pub mod symbol_aggregator;
pub mod tests;

use axum::routing::{get, post};
use axum::Router;

pub async fn start_server() -> anyhow::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let app = build_app();

    tracing::info!("🚀 Server running at http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    Ok(axum::serve(listener, app).await?)
}

pub fn build_app() -> Router {
    let app = Router::new()
        .route("/add_batch/", post(api::add_batch))
        .route("/stats/", get(api::get_stats));
    app
}
