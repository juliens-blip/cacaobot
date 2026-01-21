//! Performance metrics tracking
//!
//! Tracks bot performance metrics including:
//! - Trade history with entry/exit prices
//! - Win rate calculation
//! - P&L tracking (daily and total)
//! - Position monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Result of a completed trade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeResult {
    /// Trade closed with profit
    Win,
    /// Trade closed with loss
    Loss,
    /// Trade still open
    Open,
}

/// Individual trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID from broker
    pub id: String,
    /// Trade direction (BUY/SELL)
    pub direction: String,
    /// Volume in lots
    pub volume: f64,
    /// Entry price
    pub entry_price: f64,
    /// Exit price (None if still open)
    pub exit_price: Option<f64>,
    /// Entry timestamp
    pub entry_time: DateTime<Utc>,
    /// Exit timestamp (None if still open)
    pub exit_time: Option<DateTime<Utc>>,
    /// Profit/Loss in account currency
    pub pnl: f64,
    /// Trade result
    pub result: TradeResult,
}

impl Trade {
    /// Create a new open trade
    pub fn new(id: String, direction: String, volume: f64, entry_price: f64) -> Self {
        Self {
            id,
            direction,
            volume,
            entry_price,
            exit_price: None,
            entry_time: Utc::now(),
            exit_time: None,
            pnl: 0.0,
            result: TradeResult::Open,
        }
    }

    /// Close the trade with exit price and P&L
    pub fn close(&mut self, exit_price: f64, pnl: f64) {
        self.exit_price = Some(exit_price);
        self.exit_time = Some(Utc::now());
        self.pnl = pnl;
        self.result = if pnl > 0.0 {
            TradeResult::Win
        } else {
            TradeResult::Loss
        };
    }

    /// Check if trade is still open
    pub fn is_open(&self) -> bool {
        self.result == TradeResult::Open
    }

    /// Get duration in seconds
    pub fn duration_secs(&self) -> i64 {
        match self.exit_time {
            Some(exit) => (exit - self.entry_time).num_seconds(),
            None => (Utc::now() - self.entry_time).num_seconds(),
        }
    }
}

/// Bot performance metrics
#[derive(Debug, Clone)]
pub struct BotMetrics {
    /// Starting balance
    pub starting_balance: f64,
    /// Current balance
    pub current_balance: f64,
    /// Balance at start of today
    pub daily_starting_balance: f64,
    /// All trades (historical + open)
    pub trades: Vec<Trade>,
    /// Current RSI value
    pub current_rsi: Option<f64>,
    /// Current sentiment score
    pub current_sentiment: Option<i32>,
    /// Current FCPO price
    pub current_price: Option<f64>,
    /// Bot start time
    pub start_time: DateTime<Utc>,
}

impl BotMetrics {
    /// Create new metrics tracker
    pub fn new(starting_balance: f64) -> Self {
        Self {
            starting_balance,
            current_balance: starting_balance,
            daily_starting_balance: starting_balance,
            trades: Vec::new(),
            current_rsi: None,
            current_sentiment: None,
            current_price: None,
            start_time: Utc::now(),
        }
    }

    /// Add a new trade
    pub fn add_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
    }

    /// Close a trade by ID and update balance
    pub fn close_trade(&mut self, trade_id: &str, exit_price: f64) -> Option<f64> {
        let trade = self
            .trades
            .iter_mut()
            .find(|t| t.id == trade_id && t.is_open())?;

        let pnl = if trade.direction.eq_ignore_ascii_case("BUY") {
            (exit_price - trade.entry_price) * trade.volume
        } else if trade.direction.eq_ignore_ascii_case("SELL") {
            (trade.entry_price - exit_price) * trade.volume
        } else {
            0.0
        };

        trade.close(exit_price, pnl);
        self.current_balance += pnl;
        Some(pnl)
    }

    /// Update market data
    pub fn update_market_data(&mut self, price: f64, rsi: f64, sentiment: i32) {
        self.current_price = Some(price);
        self.current_rsi = Some(rsi);
        self.current_sentiment = Some(sentiment);
    }

    /// Update account balance
    pub fn update_balance(&mut self, balance: f64) {
        self.current_balance = balance;
    }

    /// Get total number of trades
    pub fn total_trades(&self) -> usize {
        self.trades.iter().filter(|t| !t.is_open()).count()
    }

    /// Get total number of trades (compatibility alias)
    pub fn get_total_trades(&self) -> usize {
        self.total_trades()
    }

    /// Get number of winning trades
    pub fn winning_trades(&self) -> usize {
        self.trades
            .iter()
            .filter(|t| t.result == TradeResult::Win)
            .count()
    }

    /// Get number of losing trades
    pub fn losing_trades(&self) -> usize {
        self.trades
            .iter()
            .filter(|t| t.result == TradeResult::Loss)
            .count()
    }

    /// Calculate win rate as percentage
    pub fn win_rate(&self) -> f64 {
        let total = self.total_trades();
        if total == 0 {
            return 0.0;
        }
        (self.winning_trades() as f64 / total as f64) * 100.0
    }

    /// Calculate win rate as percentage (compatibility alias)
    pub fn calculate_win_rate(&self) -> f64 {
        self.win_rate()
    }

    /// Get total P&L
    pub fn total_pnl(&self) -> f64 {
        self.current_balance - self.starting_balance
    }

    /// Get today's P&L
    pub fn daily_pnl(&self) -> f64 {
        self.current_balance - self.daily_starting_balance
    }

    /// Get today's P&L as percentage
    pub fn daily_pnl_percent(&self) -> f64 {
        if self.daily_starting_balance == 0.0 {
            return 0.0;
        }
        (self.daily_pnl() / self.daily_starting_balance) * 100.0
    }

    /// Get all open positions
    pub fn open_positions(&self) -> Vec<&Trade> {
        self.trades.iter().filter(|t| t.is_open()).collect()
    }

    /// Get all open positions (compatibility alias)
    pub fn get_open_positions(&self) -> Vec<&Trade> {
        self.open_positions()
    }

    /// Get today's trades
    pub fn todays_trades(&self) -> Vec<&Trade> {
        let today_start = match Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| dt.and_local_timezone(Utc).single())
        {
            Some(start) => start,
            None => return Vec::new(),
        };

        self.trades
            .iter()
            .filter(|t| t.entry_time >= today_start)
            .collect()
    }

    /// Get recent trades (last N)
    pub fn recent_trades(&self, count: usize) -> Vec<&Trade> {
        self.trades
            .iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Reset daily metrics (call at midnight)
    pub fn reset_daily(&mut self) {
        self.daily_starting_balance = self.current_balance;
    }

    /// Record realized P&L from closed position
    pub fn record_realized_pnl(&mut self, pnl: f64) {
        self.current_balance += pnl;
    }

    /// Add an open position to tracking
    pub fn add_open_position(&mut self, id: String, direction: String, volume: f64, entry_price: f64) {
        let trade = Trade::new(id, direction, volume, entry_price);
        self.trades.push(trade);
    }

    /// Close a tracked position
    pub fn close_position(&mut self, id: &str, exit_price: f64, pnl: f64) {
        if let Some(trade) = self.trades.iter_mut().find(|t| t.id == id && t.is_open()) {
            trade.close(exit_price, pnl);
        }
    }

    /// Get bot runtime in seconds
    pub fn runtime_secs(&self) -> i64 {
        (Utc::now() - self.start_time).num_seconds()
    }

    /// Format runtime as human-readable string
    pub fn runtime_formatted(&self) -> String {
        let secs = self.runtime_secs();
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Thread-safe metrics container
#[derive(Clone)]
pub struct MetricsHandle {
    inner: Arc<Mutex<BotMetrics>>,
}

impl MetricsHandle {
    /// Create new metrics handle
    pub fn new(starting_balance: f64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BotMetrics::new(starting_balance))),
        }
    }

    /// Execute closure with metrics read access
    pub fn with_metrics<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&BotMetrics) -> R,
    {
        let metrics = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        f(&metrics)
    }

    /// Execute closure with metrics write access
    pub fn with_metrics_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut BotMetrics) -> R,
    {
        let mut metrics = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        f(&mut metrics)
    }

    /// Clone the current metrics snapshot
    pub fn snapshot(&self) -> BotMetrics {
        match self.inner.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new("12345".to_string(), "BUY".to_string(), 0.1, 4800.0);
        assert!(trade.is_open());
        assert_eq!(trade.pnl, 0.0);
        assert_eq!(trade.result, TradeResult::Open);
    }

    #[test]
    fn test_trade_close_win() {
        let mut trade = Trade::new("12345".to_string(), "BUY".to_string(), 0.1, 4800.0);
        trade.close(4850.0, 50.0);
        assert!(!trade.is_open());
        assert_eq!(trade.result, TradeResult::Win);
        assert_eq!(trade.pnl, 50.0);
    }

    #[test]
    fn test_trade_close_loss() {
        let mut trade = Trade::new("12345".to_string(), "BUY".to_string(), 0.1, 4800.0);
        trade.close(4750.0, -50.0);
        assert!(!trade.is_open());
        assert_eq!(trade.result, TradeResult::Loss);
        assert_eq!(trade.pnl, -50.0);
    }

    #[test]
    fn test_metrics_win_rate() {
        let mut metrics = BotMetrics::new(10000.0);

        // Add 2 winning trades
        let mut trade1 = Trade::new("1".to_string(), "BUY".to_string(), 0.1, 4800.0);
        trade1.close(4850.0, 50.0);
        metrics.add_trade(trade1);

        let mut trade2 = Trade::new("2".to_string(), "BUY".to_string(), 0.1, 4900.0);
        trade2.close(4950.0, 50.0);
        metrics.add_trade(trade2);

        // Add 1 losing trade
        let mut trade3 = Trade::new("3".to_string(), "SELL".to_string(), 0.1, 4800.0);
        trade3.close(4850.0, -50.0);
        metrics.add_trade(trade3);

        assert_eq!(metrics.total_trades(), 3);
        assert_eq!(metrics.winning_trades(), 2);
        assert_eq!(metrics.losing_trades(), 1);
        assert!((metrics.win_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_metrics_pnl() {
        let mut metrics = BotMetrics::new(10000.0);
        metrics.update_balance(10500.0);

        assert_eq!(metrics.total_pnl(), 500.0);
        assert_eq!(metrics.daily_pnl(), 500.0);
        assert!((metrics.daily_pnl_percent() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_open_positions() {
        let mut metrics = BotMetrics::new(10000.0);

        // Add open trade
        let trade1 = Trade::new("1".to_string(), "BUY".to_string(), 0.1, 4800.0);
        metrics.add_trade(trade1);

        // Add closed trade
        let mut trade2 = Trade::new("2".to_string(), "BUY".to_string(), 0.1, 4900.0);
        trade2.close(4950.0, 50.0);
        metrics.add_trade(trade2);

        assert_eq!(metrics.open_positions().len(), 1);
        assert_eq!(metrics.total_trades(), 1);
    }

    #[test]
    fn test_close_trade_updates_balance() {
        let mut metrics = BotMetrics::new(10000.0);
        metrics.add_trade(Trade::new("t1".to_string(), "BUY".to_string(), 1.0, 100.0));

        let pnl = metrics.close_trade("t1", 102.0).expect("trade should close");
        assert!((pnl - 2.0).abs() < f64::EPSILON);
        assert!((metrics.current_balance - 10002.0).abs() < f64::EPSILON);
        assert_eq!(metrics.get_total_trades(), 1);
        assert_eq!(metrics.get_open_positions().len(), 0);
    }
}
