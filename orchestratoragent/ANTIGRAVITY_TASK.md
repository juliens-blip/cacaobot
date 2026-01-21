# ANTIGRAVITY TASK - Risk Metrics Module

**Date**: 2026-01-21 13:05
**Priority**: HIGH
**Status**: ASSIGNED

---

## TASK: Implement Risk Metrics (TASK-AMP-003)

Créer le module complet de métriques de risque.

### Fichier à créer: `src/modules/monitoring/risk_metrics.rs`

```rust
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

    /// Value at Risk (95% confidence)
    pub fn var_95(&self) -> Option<f64> {
        if self.returns.len() < 30 {
            return None;
        }
        let mean = self.mean_return();
        let std = self.std_deviation()?;
        Some(mean - 1.645 * std)
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
    fn test_max_drawdown() {
        let mut metrics = RiskMetrics::new(0.05);
        metrics.update_drawdown(100.0);
        metrics.update_drawdown(110.0);
        metrics.update_drawdown(95.0);
        assert!(metrics.max_drawdown() > 0.13);
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
}
```

### Intégration dans mod.rs:

Ajouter dans `/home/julien/Documents/palm-oil-bot/src/modules/monitoring/mod.rs`:
```rust
pub mod risk_metrics;
pub use risk_metrics::RiskMetrics;
```

### Validation:
- [ ] Fichier créé
- [ ] Tests passent: `cargo test risk_metrics`
- [ ] Intégré dans mod.rs

---

**Execute maintenant.**
