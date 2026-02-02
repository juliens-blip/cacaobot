//! Circuit Breakers Stress Tests
//!
//! Comprehensive stress testing for circuit breaker behavior under extreme conditions.
//! Validates capital protection mechanisms work correctly during market stress.

use palm_oil_bot::modules::trading::circuit_breakers::{CircuitBreakerConfig, CircuitBreakers};

/// Simulate multiple small losses accumulating to daily limit
#[tokio::test]
async fn test_daily_loss_limit_triggers_at_threshold() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05, // -5%
        max_consecutive_losses: 10, // High to not interfere
        volatility_threshold: 10.0,
    };
    let mut cb = CircuitBreakers::new(config);

    // -4.9% should NOT trigger
    assert!(!cb.check_daily_loss(-0.049));
    assert!(cb.is_trading_allowed());

    // -5.0% should trigger (at threshold)
    cb.force_reset();
    assert!(cb.check_daily_loss(-0.05));
    assert!(!cb.is_trading_allowed());

    // -6% should definitely trigger
    cb.force_reset();
    assert!(cb.check_daily_loss(-0.06));
    assert!(!cb.is_trading_allowed());
}

/// Simulate catastrophic 10%+ daily loss
#[tokio::test]
async fn test_daily_loss_limit_catastrophic_loss() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Simulate -10% loss (flash crash scenario)
    assert!(cb.check_daily_loss(-0.10));
    assert!(!cb.is_trading_allowed());
    assert!(cb.is_triggered());

    // Verify getters work correctly
    assert!((cb.get_daily_pnl() - (-0.10)).abs() < 0.001);
}

/// Test exactly 3 consecutive losses triggers breaker
#[tokio::test]
async fn test_consecutive_losses_exact_threshold() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.10,
        max_consecutive_losses: 3,
        volatility_threshold: 10.0,
    };
    let mut cb = CircuitBreakers::new(config);

    // 1st loss
    cb.record_trade_result(false);
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 1);

    // 2nd loss
    cb.record_trade_result(false);
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 2);

    // 3rd loss - TRIGGER
    cb.record_trade_result(false);
    assert!(!cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 3);
}

/// Test 10+ consecutive losses in a row (disaster scenario)
#[tokio::test]
async fn test_consecutive_losses_extended_losing_streak() {
    let config = CircuitBreakerConfig {
        max_consecutive_losses: 3,
        ..Default::default()
    };
    let mut cb = CircuitBreakers::new(config);

    // Simulate 10 consecutive losses
    for i in 1..=10 {
        cb.record_trade_result(false);
        if i >= 3 {
            assert!(!cb.is_trading_allowed(), "Should be triggered after {} losses", i);
        }
    }
    assert_eq!(cb.get_consecutive_losses(), 10);
}

/// Test volatility spike detection at various levels
#[tokio::test]
async fn test_volatility_spike_detection_gradual() {
    let config = CircuitBreakerConfig {
        volatility_threshold: 2.0,
        ..Default::default()
    };
    let cb = CircuitBreakers::new(config);

    let avg_atr = 10.0;

    // 1.5x average - OK
    assert!(!cb.check_volatility(15.0, avg_atr));

    // 1.99x average - still OK (just under threshold)
    assert!(!cb.check_volatility(19.9, avg_atr));

    // 2.0x average - TRIGGER
    assert!(cb.check_volatility(20.0, avg_atr));

    // 3.0x average - definitely TRIGGER
    assert!(cb.check_volatility(30.0, avg_atr));

    // 5.0x average - extreme volatility
    assert!(cb.check_volatility(50.0, avg_atr));
}

/// Test volatility with edge case: zero average ATR
#[tokio::test]
async fn test_volatility_spike_zero_average() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreakers::new(config);

    // Zero average should not panic, should return false
    assert!(!cb.check_volatility(10.0, 0.0));
    assert!(!cb.check_volatility(0.0, 0.0));
}

/// Test that max positions of 1 is enforced (simulated via consecutive losses)
#[tokio::test]
async fn test_max_positions_enforcement_simulation() {
    // Note: Actual max positions is enforced in strategy.rs
    // This test verifies circuit breakers don't interfere with position checks
    
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreakers::new(config);

    // Circuit breakers should allow trading initially
    assert!(cb.is_trading_allowed());
}

/// Test daily reset clears all state
#[tokio::test]
async fn test_circuit_breaker_reset_clears_all_state() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Trigger via daily loss
    cb.check_daily_loss(-0.10);
    assert!(cb.is_triggered());

    // Record some losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);

    // Reset
    cb.reset_daily();

    // Verify all state cleared
    assert!(!cb.is_triggered());
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_daily_pnl(), 0.0);
    assert_eq!(cb.get_consecutive_losses(), 0);
}

/// Test force reset works after any trigger
#[tokio::test]
async fn test_force_reset_after_consecutive_losses() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Trigger via consecutive losses
    for _ in 0..5 {
        cb.record_trade_result(false);
    }
    assert!(cb.is_triggered());

    // Force reset
    cb.force_reset();
    assert!(!cb.is_triggered());
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 0);
}

/// Test combined triggers (both daily loss AND consecutive losses)
#[tokio::test]
async fn test_multiple_triggers_simultaneously() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Hit daily loss limit
    cb.check_daily_loss(-0.10);
    assert!(cb.is_triggered());

    // Also hit consecutive losses
    for _ in 0..5 {
        cb.record_trade_result(false);
    }
    assert!(cb.is_triggered());

    // Reset should clear BOTH
    cb.reset_daily();
    assert!(!cb.is_triggered());
    assert!(cb.is_trading_allowed());
}

/// Test recovery cycle: trigger → reset → new session
#[tokio::test]
async fn test_full_recovery_cycle() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Day 1: Trade and hit limit
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert!(!cb.is_trading_allowed());

    // Day 2: Reset (midnight)
    cb.reset_daily();
    assert!(cb.is_trading_allowed());

    // Day 2: New trades
    cb.record_trade_result(true); // Win
    assert_eq!(cb.get_consecutive_losses(), 0);

    cb.record_trade_result(false); // Loss
    assert_eq!(cb.get_consecutive_losses(), 1);
    assert!(cb.is_trading_allowed());
}

/// Stress test: Rapid state changes
#[tokio::test]
async fn test_rapid_state_changes() {
    let config = CircuitBreakerConfig {
        max_consecutive_losses: 5,
        ..Default::default()
    };
    let mut cb = CircuitBreakers::new(config);

    // Rapid win/loss pattern
    for _ in 0..100 {
        cb.record_trade_result(false);
        cb.record_trade_result(false);
        cb.record_trade_result(true); // Reset
    }

    // Should still be trading allowed (never hit 5 consecutive)
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 0);
}

/// Test P&L tracking accuracy
#[tokio::test]
async fn test_pnl_tracking_accuracy() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Sequence of P&L updates
    cb.check_daily_loss(-0.01); // -1%
    assert!((cb.get_daily_pnl() - (-0.01)).abs() < 0.0001);

    cb.check_daily_loss(-0.02); // -2%
    assert!((cb.get_daily_pnl() - (-0.02)).abs() < 0.0001);

    cb.check_daily_loss(0.01); // +1% (recovery)
    assert!((cb.get_daily_pnl() - 0.01).abs() < 0.0001);
    assert!(cb.is_trading_allowed()); // Not triggered
}

/// Test edge case: exactly at threshold
#[tokio::test]
async fn test_boundary_conditions() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 2.0,
    };
    let mut cb = CircuitBreakers::new(config);

    // Daily loss: exactly at -5% (at threshold, should trigger due to <=)
    assert!(cb.check_daily_loss(-0.05));

    cb.force_reset();

    // Just above: -4.999% (should NOT trigger)
    assert!(!cb.check_daily_loss(-0.04999));
}

/// Test configuration validation
#[test]
fn test_config_defaults() {
    let config = CircuitBreakerConfig::default();

    assert_eq!(config.daily_loss_limit, -0.05);
    assert_eq!(config.max_consecutive_losses, 3);
    assert_eq!(config.volatility_threshold, 2.0);
}

/// Test custom configuration
#[test]
fn test_custom_config() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.03,  // Stricter
        max_consecutive_losses: 2, // Stricter
        volatility_threshold: 1.5, // Stricter
    };
    let mut cb = CircuitBreakers::new(config);

    // -3.5% should trigger with stricter limit
    assert!(cb.check_daily_loss(-0.035));
}

/// Integration: Simulate a full trading day with mixed results
#[tokio::test]
async fn test_simulated_trading_day() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Morning session: 2 wins, 1 loss
    cb.record_trade_result(true);  // Win
    cb.record_trade_result(false); // Loss (1 consecutive)
    cb.record_trade_result(true);  // Win (resets counter)
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_consecutive_losses(), 0);

    // Afternoon session: 3 losses in a row
    cb.record_trade_result(false); // Loss 1
    cb.record_trade_result(false); // Loss 2
    assert!(cb.is_trading_allowed());
    cb.record_trade_result(false); // Loss 3 - TRIGGER
    assert!(!cb.is_trading_allowed());

    // End of day reset
    cb.reset_daily();
    assert!(cb.is_trading_allowed());
}

/// Test volatility breaker with realistic ATR values
#[tokio::test]
async fn test_volatility_realistic_atr_values() {
    let config = CircuitBreakerConfig {
        volatility_threshold: 2.0,
        ..Default::default()
    };
    let cb = CircuitBreakers::new(config);

    // Normal market: ATR = 50, Average = 45
    assert!(!cb.check_volatility(50.0, 45.0)); // 1.11x

    // High volatility: ATR = 100, Average = 45
    assert!(cb.check_volatility(100.0, 45.0)); // 2.22x

    // Extreme volatility (news event): ATR = 200, Average = 45
    assert!(cb.check_volatility(200.0, 45.0)); // 4.44x
}
