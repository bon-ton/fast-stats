use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use criterion::{criterion_group, criterion_main, Criterion};
use fast_stats::build_app;
use fast_stats::symbol_aggregator::SymbolAggregator;
use serde_json::json;
use tower::ServiceExt;

fn bench_add_batch(c: &mut Criterion) {
    let mut aggregator = SymbolAggregator::<8, 10>::new();

    let values: Vec<f64> = (0..10_000).map(|i| 100.0 + (i as f64 * 0.01)).collect();

    c.bench_function("add_batch_10k", |b| {
        b.iter(|| {
            aggregator.add_batch(&values);
        })
    });
}

fn bench_get_stats(c: &mut Criterion) {
    let mut aggregator = SymbolAggregator::<8, 10>::new();

    let values = fast_stats::tests::generate_random_data(100_000_000, 3.14, 271.72, 457325.);

    for chunk in values.chunks(10_000) {
        aggregator.add_batch(chunk);
    }

    c.bench_function("get_stats_k=4", |b| {
        b.iter(|| {
            aggregator.get_stats(4).unwrap();
        })
    });

    c.bench_function("get_stats_k=8", |b| {
        b.iter(|| {
            aggregator.get_stats(8).unwrap();
        })
    });

    let values = fast_stats::tests::generate_random_data(10_000, 314.15, 27.172, 4573.25);

    c.bench_function("add_and_get_stats_k=4", |b| {
        b.iter(|| {
            aggregator.add_batch(&values);
            aggregator.get_stats(4).unwrap();
        })
    });

    c.bench_function("add_and_get_stats_k=8", |b| {
        b.iter(|| {
            aggregator.add_batch(&values);
            aggregator.get_stats(8).unwrap();
        })
    });
}

fn bench_http_add_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(async { build_app() });

    let payload = json!({
        "symbol": "ABC",
        "values": (0..10_000).map(|i| 100.0 + i as f64 * 0.01).collect::<Vec<_>>()
    });

    c.bench_function("POST /add_batch", |b| {
        b.to_async(&rt).iter(|| async {
            let body = Body::from(serde_json::to_vec(&payload).unwrap());
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/add_batch/")
                        .header("content-type", "application/json")
                        .body(body)
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::CREATED);
        });
    });
}

fn bench_http_get_stats(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(async {
        let app = build_app();

        // Seed some data first
        let payload = json!({
            "symbol": "ABC",
            "values": (0..1_000_000).map(|i| 100.0 + i as f64 * 0.01).collect::<Vec<_>>()
        });

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/add_batch/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        app
    });

    c.bench_function("GET /stats?k=7", |b| {
        b.to_async(&rt).iter(|| async {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/stats/?symbol=ABC&k=7")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        });
    });

    c.bench_function("GET /stats?k=8", |b| {
        b.to_async(&rt).iter(|| async {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/stats/?symbol=ABC&k=8")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        });
    });
}

criterion_group!(
    benches,
    bench_add_batch,
    bench_get_stats,
    bench_http_add_batch,
    bench_http_get_stats
);
criterion_main!(benches);
