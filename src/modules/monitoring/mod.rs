//! Monitoring module
//!
//! Provides real-time metrics tracking and CLI dashboard using ratatui.
//!
//! ## Components
//! - `metrics`: Performance tracking (trades, win rate, P&L)
//! - `dashboard`: Terminal UI with live data visualization
//! - `risk_metrics`: Advanced risk calculations (Sharpe, VaR, Drawdown)
//! - `circuit_breaker_status`: Real-time circuit breaker monitoring

pub mod circuit_breaker_status;
pub mod dashboard;
pub mod metrics;
pub mod risk_metrics;
pub mod prometheus;

pub use circuit_breaker_status::{BreakerInfo, BreakerState, CircuitBreakerStatus};
pub use dashboard::Dashboard;
pub use metrics::{BotMetrics, MetricsHandle, Trade, TradeResult};
pub use risk_metrics::RiskMetrics;
pub use prometheus::{start_metrics_server, metrics_enabled};
