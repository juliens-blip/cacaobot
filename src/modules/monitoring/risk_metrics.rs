//! Risk metrics module for portfolio analysis
//! Provides Sharpe, Sortino, Max Drawdown, VaR calculations

use std::collections::VecDeque;

const TRADING_DAYS: f64 = 252.0;

/// Risk metrics calculator
pub struct RiskMetrics {
    returns: VecDeque<f64>,
    max_size: usize,
    risk_free_rate: f64,
    peak_value: f64,
    current_drawdown: f64,
    max_drawdown: f64,
}

impl RiskMetrics {
    pub fn new(risk_free_rate: f64) -> Self {
        Self {
            returns: VecDeque::with_capacity(252),
            max_size: 252,
            risk_free_rate,
            peak_value: 0.0,
            current_drawdown: 0.0,
            max_drawdown: 0.0,
        }
    }

    pub fn add_return(&mut self, daily_return: f64) {
        if self.returns.len() >= self.max_size {
            self.returns.pop_front();
        }
        self.returns.push_back(daily_return);
    }

    pub fn update_drawdown(&mut self, portfolio_value: f64) {
        if portfolio_value > self.peak_value {
            self.peak_value = portfolio_value;
            self.current_drawdown = 0.0;
        } else {
            self.current_drawdown = (self.peak_value - portfolio_value) / self.peak_value;
            if self.current_drawdown > self.max_drawdown {
                self.max_drawdown = self.current_drawdown;
            }
        }
    }

    /// Sharpe Ratio = (Mean Return - Risk Free Rate) / Std Dev * sqrt(252)
    pub fn sharpe_ratio(&self) -> Option<f64> {
        if self.returns.len() < 30 {
            return None;
        }
        let mean = self.mean_return();
        let std = self.std_deviation()?;
        if std == 0.0 {
            return None;
        }
        let daily_rf = self.risk_free_rate / TRADING_DAYS;
        Some((mean - daily_rf) / std * TRADING_DAYS.sqrt())
    }

    /// Sortino Ratio - only uses downside deviation
    pub fn sortino_ratio(&self) -> Option<f64> {
        if self.returns.len() < 30 {
            return None;
        }
        let mean = self.mean_return();
        let downside_std = self.downside_deviation()?;
        if downside_std == 0.0 {
            return None;
        }
        let daily_rf = self.risk_free_rate / TRADING_DAYS;
        Some((mean - daily_rf) / downside_std * TRADING_DAYS.sqrt())
    }

    /// Maximum Drawdown
    pub fn max_drawdown(&self) -> f64 {
        self.max_drawdown
    }

    /// Current Drawdown
    pub fn current_drawdown(&self) -> f64 {
        self.current_drawdown
    }

    /// Value at Risk (95% confidence)
    pub fn var_95(&self) -> Option<f64> {
        if self.returns.len() < 30 {
            return None;
        }
        let mean = self.mean_return();
        let std = self.std_deviation()?;
        Some(mean - 1.645 * std)
    }

    /// Returns count
    pub fn returns_count(&self) -> usize {
        self.returns.len()
    }

    fn mean_return(&self) -> f64 {
        if self.returns.is_empty() {
            return 0.0;
        }
        self.returns.iter().sum::<f64>() / self.returns.len() as f64
    }

    fn std_deviation(&self) -> Option<f64> {
        if self.returns.len() < 2 {
            return None;
        }
        let mean = self.mean_return();
        let variance = self.returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / (self.returns.len() - 1) as f64;
        Some(variance.sqrt())
    }

    fn downside_deviation(&self) -> Option<f64> {
        if self.returns.len() < 2 {
            return None;
        }
        let negative_returns: Vec<f64> = self.returns.iter()
            .filter(|&&r| r < 0.0)
            .copied()
            .collect();
        if negative_returns.is_empty() {
            return Some(0.0);
        }
        let mean_neg = negative_returns.iter().sum::<f64>() / negative_returns.len() as f64;
        let variance = negative_returns.iter()
            .map(|r| (r - mean_neg).powi(2))
            .sum::<f64>() / negative_returns.len() as f64;
        Some(variance.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics() {
        let metrics = RiskMetrics::new(0.05);
        assert_eq!(metrics.returns_count(), 0);
        assert_eq!(metrics.max_drawdown(), 0.0);
    }

    #[test]
    fn test_add_returns() {
        let mut metrics = RiskMetrics::new(0.05);
        for i in 0..10 {
            metrics.add_return(0.01 * i as f64);
        }
        assert_eq!(metrics.returns_count(), 10);
    }

    #[test]
    fn test_sharpe_ratio() {
        let mut metrics = RiskMetrics::new(0.05);
        for i in 0..50 {
            metrics.add_return(0.001 + (i % 3) as f64 * 0.0005);
        }
        let sharpe = metrics.sharpe_ratio();
        assert!(sharpe.is_some());
        assert!(sharpe.unwrap() > 0.0);
    }

    #[test]
    fn test_sharpe_ratio_insufficient_data() {
        let mut metrics = RiskMetrics::new(0.05);
        for _ in 0..20 {
            metrics.add_return(0.001);
        }
        assert!(metrics.sharpe_ratio().is_none());
    }

    #[test]
    fn test_sortino_ratio() {
        let mut metrics = RiskMetrics::new(0.05);
        for i in 0..50 {
            // Mix of positive and negative returns
            let ret = if i % 4 == 0 { -0.002 } else { 0.003 };
            metrics.add_return(ret);
        }
        let sortino = metrics.sortino_ratio();
        assert!(sortino.is_some());
    }

    #[test]
    fn test_max_drawdown() {
        let mut metrics = RiskMetrics::new(0.05);
        metrics.update_drawdown(100.0);
        metrics.update_drawdown(110.0);
        metrics.update_drawdown(95.0);
        // Drawdown from 110 to 95 = 15/110 = 0.1363...
        assert!(metrics.max_drawdown() > 0.13);
        assert!(metrics.max_drawdown() < 0.14);
    }

    #[test]
    fn test_drawdown_recovery() {
        let mut metrics = RiskMetrics::new(0.05);
        metrics.update_drawdown(100.0);
        metrics.update_drawdown(80.0);  // 20% drawdown
        metrics.update_drawdown(100.0); // Recovery
        metrics.update_drawdown(120.0); // New peak
        // Max drawdown should still be 20%
        assert!((metrics.max_drawdown() - 0.20).abs() < 0.001);
        // Current drawdown should be 0
        assert_eq!(metrics.current_drawdown(), 0.0);
    }

    #[test]
    fn test_var_95() {
        let mut metrics = RiskMetrics::new(0.05);
        for _ in 0..50 {
            metrics.add_return(0.001);
        }
        let var = metrics.var_95();
        assert!(var.is_some());
    }

    #[test]
    fn test_var_95_with_volatility() {
        let mut metrics = RiskMetrics::new(0.05);
        for i in 0..50 {
            let ret = if i % 2 == 0 { 0.02 } else { -0.01 };
            metrics.add_return(ret);
        }
        let var = metrics.var_95();
        assert!(var.is_some());
        // VaR should be negative given the volatility
        assert!(var.unwrap() < 0.01);
    }

    #[test]
    fn test_rolling_window() {
        let mut metrics = RiskMetrics::new(0.05);
        // Fill beyond capacity
        for i in 0..300 {
            metrics.add_return(i as f64 * 0.0001);
        }
        // Should be capped at max_size (252)
        assert_eq!(metrics.returns_count(), 252);
    }
}
