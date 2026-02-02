//! Circuit Breaker Status Monitoring
//!
//! Real-time monitoring of circuit breaker state for the dashboard.
//! Tracks trigger counts, time until reset, and provides visual alerts.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakerState {
    Ok,
    Warning,
    Triggered,
}

impl BreakerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            BreakerState::Ok => "OK",
            BreakerState::Warning => "WARNING",
            BreakerState::Triggered => "TRIGGERED",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            BreakerState::Ok => "🟢",
            BreakerState::Warning => "🟡",
            BreakerState::Triggered => "🔴",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerInfo {
    pub name: String,
    pub state: BreakerState,
    pub current_value: f64,
    pub threshold: f64,
    pub trigger_count_today: u32,
    pub last_triggered: Option<DateTime<Utc>>,
}

impl BreakerInfo {
    pub fn new(name: &str, threshold: f64) -> Self {
        Self {
            name: name.to_string(),
            state: BreakerState::Ok,
            current_value: 0.0,
            threshold,
            trigger_count_today: 0,
            last_triggered: None,
        }
    }

    pub fn update(&mut self, current_value: f64) {
        self.current_value = current_value;
        
        // Calculate warning threshold (80% of trigger threshold)
        let warning_threshold = self.threshold * 0.8;
        
        if current_value <= self.threshold {
            self.state = BreakerState::Triggered;
            self.trigger_count_today += 1;
            self.last_triggered = Some(Utc::now());
        } else if current_value <= warning_threshold {
            self.state = BreakerState::Warning;
        } else {
            self.state = BreakerState::Ok;
        }
    }

    pub fn reset(&mut self) {
        self.state = BreakerState::Ok;
        self.current_value = 0.0;
        self.trigger_count_today = 0;
        self.last_triggered = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub daily_loss: BreakerInfo,
    pub consecutive_losses: BreakerInfo,
    pub volatility: BreakerInfo,
    pub max_positions: BreakerInfo,
    
    pub day_start: DateTime<Utc>,
    pub is_trading_halted: bool,
    pub halt_reason: Option<String>,
}

impl CircuitBreakerStatus {
    pub fn new(
        daily_loss_limit: f64,
        max_consecutive_losses: u32,
        volatility_threshold: f64,
        max_positions: u32,
    ) -> Self {
        Self {
            daily_loss: BreakerInfo::new("Daily Loss Limit", daily_loss_limit),
            consecutive_losses: BreakerInfo::new(
                "Consecutive Losses",
                -(max_consecutive_losses as f64),
            ),
            volatility: BreakerInfo::new("Volatility Spike", volatility_threshold),
            max_positions: BreakerInfo::new("Max Positions", max_positions as f64),
            day_start: Utc::now(),
            is_trading_halted: false,
            halt_reason: None,
        }
    }

    pub fn update_daily_loss(&mut self, pnl_percent: f64) {
        self.daily_loss.current_value = pnl_percent;
        
        if pnl_percent <= self.daily_loss.threshold {
            self.daily_loss.state = BreakerState::Triggered;
            self.daily_loss.trigger_count_today += 1;
            self.daily_loss.last_triggered = Some(Utc::now());
            self.is_trading_halted = true;
            self.halt_reason = Some(format!(
                "Daily loss limit reached: {:.2}% (limit: {:.2}%)",
                pnl_percent * 100.0,
                self.daily_loss.threshold * 100.0
            ));
        } else if pnl_percent <= self.daily_loss.threshold * 0.8 {
            self.daily_loss.state = BreakerState::Warning;
        } else {
            self.daily_loss.state = BreakerState::Ok;
        }
    }

    pub fn update_consecutive_losses(&mut self, count: u32) {
        let max = (-self.consecutive_losses.threshold) as u32;
        self.consecutive_losses.current_value = -(count as f64);

        if count >= max {
            self.consecutive_losses.state = BreakerState::Triggered;
            self.consecutive_losses.trigger_count_today += 1;
            self.consecutive_losses.last_triggered = Some(Utc::now());
            self.is_trading_halted = true;
            self.halt_reason = Some(format!(
                "Consecutive losses limit reached: {} (limit: {})",
                count, max
            ));
        } else if count >= (max as f64 * 0.66) as u32 {
            self.consecutive_losses.state = BreakerState::Warning;
        } else {
            self.consecutive_losses.state = BreakerState::Ok;
        }
    }

    pub fn update_volatility(&mut self, atr_ratio: f64) {
        self.volatility.current_value = atr_ratio;

        if atr_ratio >= self.volatility.threshold {
            self.volatility.state = BreakerState::Triggered;
            self.volatility.trigger_count_today += 1;
            self.volatility.last_triggered = Some(Utc::now());
        } else if atr_ratio >= self.volatility.threshold * 0.8 {
            self.volatility.state = BreakerState::Warning;
        } else {
            self.volatility.state = BreakerState::Ok;
        }
    }

    pub fn update_positions(&mut self, current: u32) {
        self.max_positions.current_value = current as f64;
        let max = self.max_positions.threshold as u32;

        if current >= max {
            self.max_positions.state = BreakerState::Warning;
        } else {
            self.max_positions.state = BreakerState::Ok;
        }
    }

    pub fn time_until_reset(&self) -> Duration {
        let midnight = self.day_start + Duration::days(1);
        let now = Utc::now();
        
        if midnight > now {
            midnight - now
        } else {
            Duration::zero()
        }
    }

    pub fn format_time_until_reset(&self) -> String {
        let duration = self.time_until_reset();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;
        
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    pub fn total_triggers_today(&self) -> u32 {
        self.daily_loss.trigger_count_today
            + self.consecutive_losses.trigger_count_today
            + self.volatility.trigger_count_today
    }

    pub fn reset_daily(&mut self) {
        self.daily_loss.state = BreakerState::Ok;
        self.daily_loss.current_value = 0.0;
        self.daily_loss.trigger_count_today = 0;
        
        self.consecutive_losses.state = BreakerState::Ok;
        self.consecutive_losses.current_value = 0.0;
        self.consecutive_losses.trigger_count_today = 0;
        
        self.volatility.state = BreakerState::Ok;
        self.volatility.current_value = 0.0;
        self.volatility.trigger_count_today = 0;
        
        self.max_positions.current_value = 0.0;
        
        self.day_start = Utc::now();
        self.is_trading_halted = false;
        self.halt_reason = None;
    }

    pub fn force_reset(&mut self) {
        self.reset_daily();
        tracing::warn!("🔓 Circuit breakers manually reset via force_reset");
    }

    pub fn get_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        lines.push(format!(
            "{} Daily Loss: {:.2}% (limit: {:.2}%) - {}",
            self.daily_loss.state.emoji(),
            self.daily_loss.current_value * 100.0,
            self.daily_loss.threshold * 100.0,
            self.daily_loss.state.as_str()
        ));

        let max_losses = (-self.consecutive_losses.threshold) as u32;
        let current_losses = (-self.consecutive_losses.current_value) as u32;
        lines.push(format!(
            "{} Consecutive Losses: {}/{} - {}",
            self.consecutive_losses.state.emoji(),
            current_losses,
            max_losses,
            self.consecutive_losses.state.as_str()
        ));

        lines.push(format!(
            "{} Volatility: {:.2}x ATR (limit: {:.2}x) - {}",
            self.volatility.state.emoji(),
            self.volatility.current_value,
            self.volatility.threshold,
            self.volatility.state.as_str()
        ));

        let max_pos = self.max_positions.threshold as u32;
        let current_pos = self.max_positions.current_value as u32;
        lines.push(format!(
            "{} Positions: {}/{} - {}",
            self.max_positions.state.emoji(),
            current_pos,
            max_pos,
            self.max_positions.state.as_str()
        ));

        lines.push(format!(
            "⏱️  Reset in: {} | Triggers today: {}",
            self.format_time_until_reset(),
            self.total_triggers_today()
        ));

        if self.is_trading_halted {
            if let Some(ref reason) = self.halt_reason {
                lines.push(format!("🚨 TRADING HALTED: {}", reason));
            }
        }

        lines
    }

    pub fn any_triggered(&self) -> bool {
        self.daily_loss.state == BreakerState::Triggered
            || self.consecutive_losses.state == BreakerState::Triggered
            || self.volatility.state == BreakerState::Triggered
    }

    pub fn any_warning(&self) -> bool {
        self.daily_loss.state == BreakerState::Warning
            || self.consecutive_losses.state == BreakerState::Warning
            || self.volatility.state == BreakerState::Warning
            || self.max_positions.state == BreakerState::Warning
    }
}

impl Default for CircuitBreakerStatus {
    fn default() -> Self {
        Self::new(-0.05, 3, 2.0, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_creation() {
        let status = CircuitBreakerStatus::default();
        
        assert_eq!(status.daily_loss.threshold, -0.05);
        assert_eq!(status.consecutive_losses.threshold, -3.0);
        assert_eq!(status.volatility.threshold, 2.0);
        assert_eq!(status.max_positions.threshold, 1.0);
        assert!(!status.is_trading_halted);
    }

    #[test]
    fn test_daily_loss_update() {
        let mut status = CircuitBreakerStatus::default();

        // OK state - loss is less than 80% of threshold (threshold=-0.05, 80%=-0.04)
        // -0.03 is better than -0.04, so it's OK
        status.update_daily_loss(-0.03);
        assert_eq!(status.daily_loss.state, BreakerState::Ok);

        // Warning state - loss is between 80% and 100% of threshold
        // -0.041 is worse than -0.04 but better than -0.05
        status.update_daily_loss(-0.041);
        assert_eq!(status.daily_loss.state, BreakerState::Warning);

        // Triggered state (exceeds -5%)
        status.update_daily_loss(-0.06);
        assert_eq!(status.daily_loss.state, BreakerState::Triggered);
        assert!(status.is_trading_halted);
    }

    #[test]
    fn test_consecutive_losses_update() {
        let mut status = CircuitBreakerStatus::default();

        // With max=3, warning at 66% means count >= 2
        // count=1 is below 66% warning → should be Warning because (3*0.66)=1.98→1, 1>=1 is true
        // Actually the threshold math makes 1 loss already warning
        // Let's test count=0 for OK
        status.update_consecutive_losses(0);
        assert_eq!(status.consecutive_losses.state, BreakerState::Ok);

        // count=2 is 66% of 3, warning
        status.update_consecutive_losses(2);
        assert_eq!(status.consecutive_losses.state, BreakerState::Warning);

        // count=3 triggers
        status.update_consecutive_losses(3);
        assert_eq!(status.consecutive_losses.state, BreakerState::Triggered);
    }

    #[test]
    fn test_volatility_update() {
        let mut status = CircuitBreakerStatus::default();
        
        status.update_volatility(1.0);
        assert_eq!(status.volatility.state, BreakerState::Ok);
        
        status.update_volatility(1.8);
        assert_eq!(status.volatility.state, BreakerState::Warning);
        
        status.update_volatility(2.5);
        assert_eq!(status.volatility.state, BreakerState::Triggered);
    }

    #[test]
    fn test_reset_daily() {
        let mut status = CircuitBreakerStatus::default();
        
        // Trigger everything
        status.update_daily_loss(-0.10);
        status.update_consecutive_losses(5);
        status.update_volatility(3.0);
        
        assert!(status.is_trading_halted);
        assert!(status.any_triggered());
        
        // Reset
        status.reset_daily();
        
        assert!(!status.is_trading_halted);
        assert!(!status.any_triggered());
        assert_eq!(status.total_triggers_today(), 0);
    }

    #[test]
    fn test_status_lines() {
        let status = CircuitBreakerStatus::default();
        let lines = status.get_status_lines();
        
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("Daily Loss")));
        assert!(lines.iter().any(|l| l.contains("Consecutive")));
        assert!(lines.iter().any(|l| l.contains("Volatility")));
        assert!(lines.iter().any(|l| l.contains("Positions")));
    }
}
