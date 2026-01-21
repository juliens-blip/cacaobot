//! Monitoring module
//!
//! Provides real-time metrics tracking and CLI dashboard using ratatui.
//!
//! ## Components
//! - `metrics`: Performance tracking (trades, win rate, P&L)
//! - `dashboard`: Terminal UI with live data visualization

pub mod dashboard;
pub mod metrics;
pub mod risk_metrics;

pub use dashboard::Dashboard;
pub use metrics::{BotMetrics, MetricsHandle, Trade, TradeResult};
pub use risk_metrics::RiskMetrics;
