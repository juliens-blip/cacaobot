use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub daily_loss_limit: f64,      // Pourcentage (ex: -0.05 pour -5%)
    pub max_consecutive_losses: u32,
    pub volatility_threshold: f64,   // Multiplier ATR (ex: 2.0 = bloquer si ATR > 2x moyenne)
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            daily_loss_limit: -0.05,  // -5%
            max_consecutive_losses: 3,
            volatility_threshold: 2.0,
        }
    }
}

#[derive(Debug)]
pub struct CircuitBreakers {
    daily_loss_limit: f64,
    max_consecutive_losses: u32,
    volatility_threshold: f64,

    // State
    daily_pnl: f64,
    consecutive_losses: u32,
    is_triggered: bool,
}

impl CircuitBreakers {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            daily_loss_limit: config.daily_loss_limit,
            max_consecutive_losses: config.max_consecutive_losses,
            volatility_threshold: config.volatility_threshold,
            daily_pnl: 0.0,
            consecutive_losses: 0,
            is_triggered: false,
        }
    }

    /// Vérifie si la perte journalière dépasse la limite
    pub fn check_daily_loss(&mut self, pnl: f64) -> bool {
        self.daily_pnl = pnl;
        
        if pnl <= self.daily_loss_limit {
            self.is_triggered = true;
            tracing::warn!(
                "⚠️ Circuit breaker triggered: Daily loss {:.2}% exceeds limit {:.2}%",
                pnl * 100.0,
                self.daily_loss_limit * 100.0
            );
            true
        } else {
            false
        }
    }

    /// Enregistre le résultat d'un trade (gain ou perte)
    pub fn record_trade_result(&mut self, won: bool) {
        if won {
            self.consecutive_losses = 0;
        } else {
            self.consecutive_losses += 1;
            
            if self.consecutive_losses >= self.max_consecutive_losses {
                self.is_triggered = true;
                tracing::warn!(
                    "⚠️ Circuit breaker triggered: {} consecutive losses (max: {})",
                    self.consecutive_losses,
                    self.max_consecutive_losses
                );
            }
        }
    }

    /// Vérifie si la volatilité est anormalement élevée
    pub fn check_volatility(&self, atr: f64, avg_atr: f64) -> bool {
        if avg_atr == 0.0 {
            return false;
        }

        let ratio = atr / avg_atr;
        if ratio >= self.volatility_threshold {
            tracing::warn!(
                "⚠️ High volatility detected: ATR ratio {:.2} > threshold {:.2}",
                ratio,
                self.volatility_threshold
            );
            true
        } else {
            false
        }
    }

    /// Vérifie si le trading est autorisé (aucun circuit breaker déclenché)
    pub fn is_trading_allowed(&self) -> bool {
        !self.is_triggered
    }

    /// Reset quotidien (appeler à minuit)
    pub fn reset_daily(&mut self) {
        self.daily_pnl = 0.0;
        self.consecutive_losses = 0;
        self.is_triggered = false;
        tracing::info!("🔄 Circuit breakers reset for new day");
    }

    /// Force reset (pour tests ou intervention manuelle)
    pub fn force_reset(&mut self) {
        self.reset_daily();
        tracing::warn!("🔓 Circuit breakers manually reset");
    }

    /// Getters pour monitoring
    pub fn get_daily_pnl(&self) -> f64 {
        self.daily_pnl
    }

    pub fn get_consecutive_losses(&self) -> u32 {
        self.consecutive_losses
    }

    pub fn is_triggered(&self) -> bool {
        self.is_triggered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_loss_trigger() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreakers::new(config);

        // Perte de -3% (OK)
        assert!(!cb.check_daily_loss(-0.03));
        assert!(cb.is_trading_allowed());

        // Perte de -6% (TRIGGER)
        assert!(cb.check_daily_loss(-0.06));
        assert!(!cb.is_trading_allowed());
    }

    #[test]
    fn test_consecutive_losses_trigger() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreakers::new(config);

        // 2 pertes consécutives (OK)
        cb.record_trade_result(false);
        cb.record_trade_result(false);
        assert!(cb.is_trading_allowed());
        assert_eq!(cb.get_consecutive_losses(), 2);

        // 3ème perte (TRIGGER)
        cb.record_trade_result(false);
        assert!(!cb.is_trading_allowed());
        assert_eq!(cb.get_consecutive_losses(), 3);
    }

    #[test]
    fn test_consecutive_losses_reset_on_win() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreakers::new(config);

        cb.record_trade_result(false);
        cb.record_trade_result(false);
        assert_eq!(cb.get_consecutive_losses(), 2);

        // Gain = reset du compteur
        cb.record_trade_result(true);
        assert_eq!(cb.get_consecutive_losses(), 0);
    }

    #[test]
    fn test_volatility_spike_detection() {
        let config = CircuitBreakerConfig {
            volatility_threshold: 2.0,
            ..Default::default()
        };
        let cb = CircuitBreakers::new(config);

        // ATR normal (1.5x moyenne)
        assert!(!cb.check_volatility(15.0, 10.0));

        // ATR élevé (2.5x moyenne)
        assert!(cb.check_volatility(25.0, 10.0));
    }

    #[test]
    fn test_reset_functionality() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreakers::new(config);

        // Trigger circuit breaker
        cb.check_daily_loss(-0.06);
        cb.record_trade_result(false);
        cb.record_trade_result(false);
        cb.record_trade_result(false);
        assert!(!cb.is_trading_allowed());

        // Reset
        cb.reset_daily();
        assert!(cb.is_trading_allowed());
        assert_eq!(cb.get_daily_pnl(), 0.0);
        assert_eq!(cb.get_consecutive_losses(), 0);
    }

    #[test]
    fn test_force_reset() {
        let config = CircuitBreakerConfig::default();
        let mut cb = CircuitBreakers::new(config);

        cb.check_daily_loss(-0.10);
        assert!(!cb.is_trading_allowed());

        cb.force_reset();
        assert!(cb.is_trading_allowed());
    }
}
