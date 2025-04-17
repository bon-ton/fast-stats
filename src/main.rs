mod api;
mod app_state;
mod kahan;
// mod monotonic_queue;
mod shared_monotonic_queue;
mod symbol_aggregator;
mod tests;

use api::{add_batch, get_stats};
use axum::{
    routing::{get, post},
    Router,
};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/add_batch/", post(add_batch))
        .route("/stats/", get(get_stats));

    tracing::info!("🚀 Server running at http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
