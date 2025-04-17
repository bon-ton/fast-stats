use crate::app_state::SYMBOLS;
use crate::symbol_aggregator::SymbolAggregator;
use axum::{
    extract::{Json, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Mutex;

#[derive(Deserialize)]
pub struct AddBatchRequest {
    pub symbol: String,
    pub values: Vec<f64>,
}

pub async fn add_batch(Json(payload): Json<AddBatchRequest>) -> impl IntoResponse {
    tracing::info!(
        "POST /add_batch/ - symbol: {}, values: {}",
        payload.symbol,
        payload.values.len()
    );
    if payload.values.len() > 10_000 {
        return (StatusCode::BAD_REQUEST, "Too many values").into_response();
    }

    let entry = SYMBOLS
        .entry(payload.symbol.clone())
        .or_insert_with(|| Mutex::new(SymbolAggregator::new()));

    let mut agg = entry.lock().unwrap();
    agg.add_batch(&payload.values);

    (StatusCode::CREATED, Json(json!({ "status": "ok" }))).into_response()
}

#[derive(Deserialize)]
pub struct StatsRequest {
    pub symbol: String,
    pub k: u32,
}

// the output to our `create_user` handler
#[derive(Serialize)]
pub struct StatsResult {
    pub min: f64,
    pub max: f64,
    pub last: f64,
    pub avg: f64,
    pub var: f64,
}
pub async fn get_stats(Query(req): Query<StatsRequest>) -> impl IntoResponse {
    tracing::info!("GET /stats/ - symbol: {}, k: {}", req.symbol, req.k);

    if let Some(entry) = SYMBOLS.get(&req.symbol) {
        let mut agg = entry.lock().unwrap();
        if let Some(stats) = agg.get_stats(req.k) {
            return Json(stats).into_response();
        }
    }

    let msg = format!("symbol {} not found or insufficient data", req.symbol);
    tracing::warn!("{msg}");
    (StatusCode::NOT_FOUND, msg).into_response()
}
