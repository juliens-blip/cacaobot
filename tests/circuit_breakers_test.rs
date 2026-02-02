//! Circuit Breakers Tests
//!
//! Tests for the trading circuit breaker system.

use palm_oil_bot::modules::trading::circuit_breakers::{CircuitBreakerConfig, CircuitBreakers};

#[test]
fn test_daily_loss_limit() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05, // -5%
        max_consecutive_losses: 3,
        volatility_threshold: 5.0,
    };
    let mut breakers = CircuitBreakers::new(config);

    // -4% loss should be OK
    assert!(!breakers.check_daily_loss(-0.04));
    assert!(breakers.is_trading_allowed());

    // -6% loss should trigger
    assert!(breakers.check_daily_loss(-0.06));
    assert!(!breakers.is_trading_allowed());
}

#[test]
fn test_consecutive_losses() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 5.0,
    };
    let mut breakers = CircuitBreakers::new(config);

    // 2 losses should be OK
    breakers.record_trade_result(false);
    breakers.record_trade_result(false);
    assert!(breakers.is_trading_allowed());
    assert_eq!(breakers.get_consecutive_losses(), 2);

    // 3rd loss should trigger
    breakers.record_trade_result(false);
    assert!(!breakers.is_trading_allowed());
    assert_eq!(breakers.get_consecutive_losses(), 3);
}

#[test]
fn test_volatility_spike() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 2.0, // 2x average ATR
    };
    let breakers = CircuitBreakers::new(config);

    // 1.5x average should be OK
    assert!(!breakers.check_volatility(15.0, 10.0));

    // 2.5x average should trigger warning
    assert!(breakers.check_volatility(25.0, 10.0));
}

#[test]
fn test_reset_daily() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 5.0,
    };
    let mut breakers = CircuitBreakers::new(config);

    // Trigger circuit breaker
    breakers.check_daily_loss(-0.10);
    breakers.record_trade_result(false);
    breakers.record_trade_result(false);
    breakers.record_trade_result(false);
    assert!(!breakers.is_trading_allowed());

    // Reset should clear everything
    breakers.reset_daily();
    assert!(breakers.is_trading_allowed());
    assert_eq!(breakers.get_daily_pnl(), 0.0);
    assert_eq!(breakers.get_consecutive_losses(), 0);
}

#[test]
fn test_win_resets_consecutive_losses() {
    let config = CircuitBreakerConfig::default();
    let mut breakers = CircuitBreakers::new(config);

    // Accumulate losses
    breakers.record_trade_result(false);
    breakers.record_trade_result(false);
    assert_eq!(breakers.get_consecutive_losses(), 2);

    // Win should reset counter
    breakers.record_trade_result(true);
    assert_eq!(breakers.get_consecutive_losses(), 0);
}

#[test]
fn test_force_reset() {
    let config = CircuitBreakerConfig::default();
    let mut breakers = CircuitBreakers::new(config);

    // Trigger
    breakers.check_daily_loss(-0.10);
    assert!(!breakers.is_trading_allowed());

    // Force reset
    breakers.force_reset();
    assert!(breakers.is_trading_allowed());
}
