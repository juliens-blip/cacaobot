//! Prometheus metrics exporter for bot runtime metrics.

use axum::{routing::get, Router};
use chrono::Utc;
use prometheus::{Encoder, Gauge, Registry, TextEncoder};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::modules::monitoring::MetricsHandle;

#[derive(Clone)]
struct PrometheusExporter {
    registry: Registry,
    metrics: MetricsHandle,
    bot_balance: Gauge,
    bot_total_pnl: Gauge,
    bot_daily_pnl: Gauge,
    bot_win_rate: Gauge,
    bot_open_positions: Gauge,
    bot_total_trades: Gauge,
    bot_current_price: Gauge,
    bot_current_rsi: Gauge,
    bot_current_sentiment: Gauge,
    bot_runtime_seconds: Gauge,
}

impl PrometheusExporter {
    fn new(metrics: MetricsHandle) -> Self {
        let registry = Registry::new();

        let bot_balance = create_gauge("bot_current_balance", "Current account balance");
        let bot_total_pnl = create_gauge("bot_total_pnl", "Total P&L");
        let bot_daily_pnl = create_gauge("bot_daily_pnl", "Daily P&L");
        let bot_win_rate = create_gauge("bot_win_rate", "Win rate percentage");
        let bot_open_positions = create_gauge("bot_open_positions", "Open positions");
        let bot_total_trades = create_gauge("bot_total_trades", "Closed trades count");
        let bot_current_price = create_gauge("bot_current_price", "Current price");
        let bot_current_rsi = create_gauge("bot_current_rsi", "Current RSI");
        let bot_current_sentiment = create_gauge("bot_current_sentiment", "Current sentiment");
        let bot_runtime_seconds = create_gauge("bot_runtime_seconds", "Runtime in seconds");

        for gauge in [
            bot_balance.clone(),
            bot_total_pnl.clone(),
            bot_daily_pnl.clone(),
            bot_win_rate.clone(),
            bot_open_positions.clone(),
            bot_total_trades.clone(),
            bot_current_price.clone(),
            bot_current_rsi.clone(),
            bot_current_sentiment.clone(),
            bot_runtime_seconds.clone(),
        ] {
            if let Err(err) = registry.register(Box::new(gauge)) {
                warn!("Failed to register Prometheus gauge: {}", err);
            }
        }

        Self {
            registry,
            metrics,
            bot_balance,
            bot_total_pnl,
            bot_daily_pnl,
            bot_win_rate,
            bot_open_positions,
            bot_total_trades,
            bot_current_price,
            bot_current_rsi,
            bot_current_sentiment,
            bot_runtime_seconds,
        }
    }

    fn update_from_snapshot(&self) {
        let snapshot = self.metrics.snapshot();
        self.bot_balance.set(snapshot.current_balance);
        self.bot_total_pnl.set(snapshot.total_pnl());
        self.bot_daily_pnl.set(snapshot.daily_pnl());
        self.bot_win_rate.set(snapshot.win_rate());
        self.bot_open_positions
            .set(snapshot.open_positions().len() as f64);
        self.bot_total_trades
            .set(snapshot.total_trades() as f64);
        self.bot_current_price
            .set(snapshot.current_price.unwrap_or(0.0));
        self.bot_current_rsi
            .set(snapshot.current_rsi.unwrap_or(0.0));
        self.bot_current_sentiment
            .set(snapshot.current_sentiment.unwrap_or(0) as f64);
        let runtime = (Utc::now() - snapshot.start_time).num_seconds();
        self.bot_runtime_seconds.set(runtime as f64);
    }

    fn render(&self) -> String {
        self.update_from_snapshot();
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
            warn!("Failed to encode metrics: {}", err);
        }
        String::from_utf8_lossy(&buffer).to_string()
    }
}

fn create_gauge(name: &str, help: &str) -> Gauge {
    Gauge::new(name, help).unwrap_or_else(|err| {
        warn!("Failed to create gauge {}: {}", name, err);
        Gauge::new(
            "metric_init_failed",
            "Metric initialization failure (fallback)",
        )
        .unwrap_or_else(|fallback_err| {
            warn!(
                "Failed to create fallback gauge for {}: {}",
                name, fallback_err
            );
            Gauge::new(
                "metric_init_failed_2",
                "Metric initialization failure (fallback)",
            )
            .unwrap_or_else(|final_err| {
                warn!(
                    "Failed to create final fallback gauge for {}: {}",
                    name, final_err
                );
                Gauge::new(
                    "metric_init_failed_3",
                    "Metric initialization failure (fallback)",
                )
                .expect("fallback gauge")
            })
        })
    })
}

async fn metrics_handler(exporter: Arc<PrometheusExporter>) -> String {
    exporter.render()
}

pub fn start_metrics_server(metrics: MetricsHandle) -> JoinHandle<()> {
    let exporter = Arc::new(PrometheusExporter::new(metrics));
    let app = Router::new().route("/metrics", get({
        let exporter = exporter.clone();
        move || metrics_handler(exporter.clone())
    }));

    let addr = metrics_bind_addr();
    info!("Starting metrics server on {}", addr);

    tokio::spawn(async move {
        if let Err(err) = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
        {
            warn!("Metrics server stopped: {}", err);
        }
    })
}

fn metrics_bind_addr() -> SocketAddr {
    let host = std::env::var("METRICS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("METRICS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9090);
    format!("{}:{}", host, port)
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:9090".parse().expect("valid addr"))
}

pub fn metrics_enabled() -> bool {
    matches!(
        std::env::var("METRICS_ENABLED").as_deref(),
        Ok("true") | Ok("1") | Ok("yes")
    )
}
