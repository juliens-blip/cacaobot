use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    trades: Vec<TradeReturn>,
    risk_free_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeReturn {
    timestamp: DateTime<Utc>,
    pnl: f64,
    percentage_return: f64,
}

impl RiskMetrics {
    pub fn new(risk_free_rate: f64) -> Self {
        Self {
            trades: Vec::new(),
            risk_free_rate,
        }
    }

    pub fn add_trade(&mut self, pnl: f64, entry_price: f64) {
        let percentage_return = (pnl / entry_price) * 100.0;
        self.trades.push(TradeReturn {
            timestamp: Utc::now(),
            pnl,
            percentage_return,
        });
    }

    pub fn sharpe_ratio(&self) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }

        let returns: Vec<f64> = self.trades.iter().map(|t| t.percentage_return).collect();
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        
        let variance = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            return 0.0;
        }

        (mean_return - self.risk_free_rate) / std_dev
    }

    pub fn sortino_ratio(&self) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }

        let returns: Vec<f64> = self.trades.iter().map(|t| t.percentage_return).collect();
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .copied()
            .collect();
        
        if downside_returns.is_empty() {
            return f64::INFINITY;
        }

        let downside_variance = downside_returns.iter()
            .map(|r| r.powi(2))
            .sum::<f64>() / downside_returns.len() as f64;
        
        let downside_dev = downside_variance.sqrt();
        
        if downside_dev == 0.0 {
            return f64::INFINITY;
        }

        (mean_return - self.risk_free_rate) / downside_dev
    }

    pub fn max_drawdown(&self) -> (f64, f64) {
        if self.trades.is_empty() {
            return (0.0, 0.0);
        }

        let mut cumulative = 0.0;
        let mut peak = 0.0;
        let mut max_dd = 0.0;
        let mut max_dd_pct = 0.0;

        for trade in &self.trades {
            cumulative += trade.pnl;
            peak = f64::max(peak, cumulative);
            
            let drawdown = peak - cumulative;
            let drawdown_pct = if peak != 0.0 {
                (drawdown / peak.abs()) * 100.0
            } else {
                0.0
            };

            max_dd = f64::max(max_dd, drawdown);
            max_dd_pct = f64::max(max_dd_pct, drawdown_pct);
        }

        (max_dd, max_dd_pct)
    }

    pub fn value_at_risk(&self, confidence_level: f64) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }

        let mut returns: Vec<f64> = self.trades.iter()
            .map(|t| t.percentage_return)
            .collect();
        
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        let index = index.min(returns.len() - 1);
        
        returns[index].abs()
    }

    pub fn expected_shortfall(&self, confidence_level: f64) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }

        let mut returns: Vec<f64> = self.trades.iter()
            .map(|t| t.percentage_return)
            .collect();
        
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let cutoff_index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        
        if cutoff_index == 0 {
            return returns[0].abs();
        }

        let tail_losses: Vec<f64> = returns.iter()
            .take(cutoff_index)
            .copied()
            .collect();
        
        if tail_losses.is_empty() {
            return 0.0;
        }

        (tail_losses.iter().sum::<f64>() / tail_losses.len() as f64).abs()
    }

    pub fn win_loss_ratio(&self) -> f64 {
        let wins: Vec<&TradeReturn> = self.trades.iter().filter(|t| t.pnl > 0.0).collect();
        let losses: Vec<&TradeReturn> = self.trades.iter().filter(|t| t.pnl < 0.0).collect();

        if losses.is_empty() {
            return f64::INFINITY;
        }

        let avg_win = if wins.is_empty() {
            0.0
        } else {
            wins.iter().map(|t| t.pnl).sum::<f64>() / wins.len() as f64
        };

        let avg_loss = losses.iter().map(|t| t.pnl.abs()).sum::<f64>() / losses.len() as f64;

        if avg_loss == 0.0 {
            return f64::INFINITY;
        }

        avg_win / avg_loss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharpe_ratio() {
        let mut metrics = RiskMetrics::new(0.01);
        metrics.add_trade(100.0, 1000.0);
        metrics.add_trade(150.0, 1000.0);
        metrics.add_trade(-50.0, 1000.0);
        
        let sharpe = metrics.sharpe_ratio();
        assert!(sharpe > 0.0);
    }

    #[test]
    fn test_sortino_ratio() {
        let mut metrics = RiskMetrics::new(0.01);
        metrics.add_trade(100.0, 1000.0);
        metrics.add_trade(150.0, 1000.0);
        metrics.add_trade(-50.0, 1000.0);
        
        let sortino = metrics.sortino_ratio();
        assert!(sortino > metrics.sharpe_ratio());
    }

    #[test]
    fn test_max_drawdown() {
        let mut metrics = RiskMetrics::new(0.01);
        metrics.add_trade(100.0, 1000.0);
        metrics.add_trade(-200.0, 1000.0);
        metrics.add_trade(50.0, 1000.0);
        
        let (dd_abs, dd_pct) = metrics.max_drawdown();
        assert!(dd_abs > 0.0);
        assert!(dd_pct > 0.0);
    }

    #[test]
    fn test_value_at_risk_95() {
        let mut metrics = RiskMetrics::new(0.01);
        for i in 0..100 {
            metrics.add_trade((i as f64 - 50.0) * 10.0, 1000.0);
        }
        
        let var_95 = metrics.value_at_risk(0.95);
        assert!(var_95 > 0.0);
    }

    #[test]
    fn test_expected_shortfall() {
        let mut metrics = RiskMetrics::new(0.01);
        for i in 0..100 {
            metrics.add_trade((i as f64 - 50.0) * 10.0, 1000.0);
        }
        
        let es = metrics.expected_shortfall(0.95);
        let var = metrics.value_at_risk(0.95);
        assert!(es >= var);
    }

    #[test]
    fn test_win_loss_ratio() {
        let mut metrics = RiskMetrics::new(0.01);
        metrics.add_trade(200.0, 1000.0);
        metrics.add_trade(200.0, 1000.0);
        metrics.add_trade(-100.0, 1000.0);
        
        let ratio = metrics.win_loss_ratio();
        assert!(ratio == 2.0);
    }
}
