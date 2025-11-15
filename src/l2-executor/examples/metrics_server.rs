//! HTTP Metrics Server
//!
//! Provides Prometheus /metrics endpoint for L2 Executor monitoring.

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use l2_executor::metrics::SharedMetrics;
use std::sync::Arc;

/// Handler for /metrics endpoint
async fn metrics_handler(metrics: web::Data<SharedMetrics>) -> impl Responder {
    let prometheus_output = metrics.export_prometheus();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(prometheus_output)
}

/// Handler for /health endpoint
async fn health_handler() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "l2-executor",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Handler for root endpoint
async fn index_handler() -> impl Responder {
    HttpResponse::Ok().body(
        r#"
        <html>
        <head><title>L2 Executor Metrics</title></head>
        <body>
            <h1>L2 Executor Monitoring</h1>
            <ul>
                <li><a href="/metrics">Prometheus Metrics</a></li>
                <li><a href="/health">Health Check</a></li>
            </ul>
            <p>Ready for Prometheus scraping at <code>/metrics</code></p>
        </body>
        </html>
        "#,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Create shared metrics collector
    let metrics = l2_executor::metrics::create_shared_metrics();
    let metrics_data = web::Data::new(metrics.clone());

    let bind_addr = "0.0.0.0:9090";
    log::info!("🚀 Starting L2 Executor Metrics Server on {}", bind_addr);
    log::info!("📊 Prometheus endpoint: http://{}/metrics", bind_addr);
    log::info!("❤️  Health check: http://{}/health", bind_addr);

    // Start background metrics simulation (for demo)
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        simulate_metrics(metrics_clone).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(metrics_data.clone())
            .route("/", web::get().to(index_handler))
            .route("/metrics", web::get().to(metrics_handler))
            .route("/health", web::get().to(health_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
}

/// Simulate metrics updates for demonstration
async fn simulate_metrics(metrics: SharedMetrics) {
    use std::time::Duration;
    use tokio::time::sleep;

    let mut iteration = 0u64;

    loop {
        sleep(Duration::from_secs(5)).await;
        iteration += 1;

        // Simulate aggregation operation
        let proof_count = match iteration % 4 {
            0 => 6,
            1 => 25,
            2 => 150,
            _ => 800,
        };

        let strategy = if proof_count < 51 {
            "single"
        } else if proof_count < 501 {
            "two_level"
        } else {
            "three_level"
        };

        metrics.record_aggregation(strategy, proof_count, 10 + iteration % 50);

        // Simulate cache operations
        for _ in 0..10 {
            let hit = iteration % 3 != 0;
            metrics.record_cache(hit);
        }

        // Update performance metrics
        let tps = match strategy {
            "single" => 245 + (iteration % 100) as u64,
            "two_level" => 3061 + (iteration % 500) as u64,
            _ => 32653 + (iteration % 1000) as u64,
        };
        metrics.update_tps(tps, tps - 50);

        let gas_savings = match strategy {
            "single" => 83,
            "two_level" => 98,
            _ => 99,
        };
        metrics.update_savings(gas_savings, gas_savings - 5);

        // Update system metrics
        let workers = 8 + (iteration % 4) as u64;
        metrics.update_workers(workers);

        log::info!(
            "📈 Iteration {}: {} proofs, {} strategy, TPS {}, {}% gas savings",
            iteration,
            proof_count,
            strategy,
            tps,
            gas_savings
        );
    }
}
