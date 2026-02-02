// Circuit Breakers Live Validation Tests
//
// Tests validating circuit breaker behavior in live scenarios

use palm_oil_bot::modules::trading::circuit_breakers::{CircuitBreakers, CircuitBreakerConfig};

#[tokio::test]
async fn test_daily_loss_limit_triggers() {
    // SCENARIO: Simulate -5% daily loss hitting limit
    
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,  // -5%
        max_consecutive_losses: 3,
        volatility_threshold: 3.0,
    };
    let mut cb = CircuitBreakers::new(config);

    // Simulate -5.5% loss
    cb.check_daily_loss(-0.055);

    assert!(cb.is_triggered(), "Should trip at -5.5%");
    assert!(!cb.is_trading_allowed(), "Trading should be blocked");
}

#[tokio::test]
async fn test_consecutive_losses_cooldown() {
    // SCENARIO: 3 consecutive losses trigger circuit breaker
    
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Record 3 losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    cb.record_trade_result(false);

    assert!(cb.is_triggered(), "Should trip after 3 losses");
    assert_eq!(cb.get_consecutive_losses(), 3);
}

#[tokio::test]
async fn test_recovery_after_reset() {
    // SCENARIO: Circuit breaker can be reset
    
    let mut cb = CircuitBreakers::new(CircuitBreakerConfig::default());

    // Trigger
    cb.check_daily_loss(-0.10);
    assert!(cb.is_triggered());

    // Reset
    cb.reset_daily();
    assert!(!cb.is_triggered(), "Should not be triggered after reset");
    assert!(cb.is_trading_allowed());
}

#[tokio::test]
async fn test_volatility_spike_detection() {
    // SCENARIO: High volatility triggers circuit breaker
    
    let config = CircuitBreakerConfig {
        volatility_threshold: 2.0,
        ..Default::default()
    };
    let cb = CircuitBreakers::new(config);

    // ATR is 2.5x average (exceeds 2.0 threshold)
    let result = cb.check_volatility(25.0, 10.0);
    assert!(result, "Should detect volatility spike");
}

#[tokio::test]
async fn test_winning_trade_resets_consecutive_losses() {
    // SCENARIO: Win resets consecutive loss counter
    
    let mut cb = CircuitBreakers::new(CircuitBreakerConfig::default());

    // 2 losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 2);

    // 1 win resets counter
    cb.record_trade_result(true);
    assert_eq!(cb.get_consecutive_losses(), 0);

    // 2 more losses shouldn't trigger (counter was reset)
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert!(!cb.is_triggered());
}

#[test]
fn test_threshold_configuration() {
    // Verify different thresholds work correctly
    
    let strict_config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 2.0,
    };
    
    let lenient_config = CircuitBreakerConfig {
        daily_loss_limit: -0.10,
        max_consecutive_losses: 5,
        volatility_threshold: 5.0,
    };

    let mut cb_strict = CircuitBreakers::new(strict_config);
    let mut cb_lenient = CircuitBreakers::new(lenient_config);

    // -6% loss
    cb_strict.check_daily_loss(-0.06);
    cb_lenient.check_daily_loss(-0.06);

    assert!(cb_strict.is_triggered(), "Strict should trip at -6%");
    assert!(!cb_lenient.is_triggered(), "Lenient should not trip at -6% (limit -10%)");
}

#[tokio::test]
async fn test_state_persistence_documented() {
    // NOTE: This test documents expected future behavior
    // TODO: Implement circuit breaker state persistence
    
    let mut cb = CircuitBreakers::new(CircuitBreakerConfig::default());
    
    // Trigger
    for _ in 0..3 {
        cb.record_trade_result(false);
    }
    assert!(cb.is_triggered());

    // Future: Save state to disk
    // Future: Load state after restart
    // Future: Verify state persisted correctly
}

#[tokio::test]
async fn test_force_reset_manual_intervention() {
    // SCENARIO: Manual override via force_reset
    
    let mut cb = CircuitBreakers::new(CircuitBreakerConfig::default());

    // Trigger daily loss
    cb.check_daily_loss(-0.10);
    assert!(cb.is_triggered());

    // Force reset (manual intervention)
    cb.force_reset();
    assert!(!cb.is_triggered());
    assert!(cb.is_trading_allowed());
}
