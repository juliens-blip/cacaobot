use palm_oil_bot::config::{StrategyConfig, TradingConfig};
use palm_oil_bot::modules::trading::{CircuitBreakers, TradingStrategy, OrderSide, CloseReason, Position};
use palm_oil_bot::modules::trading::circuit_breakers::CircuitBreakerConfig;

#[test]
fn test_daily_loss_limit_triggers() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05, // -5%
        max_consecutive_losses: 3,
        volatility_threshold: 2.0,
    };
    let mut cb = CircuitBreakers::new(config);

    // Trading allowed initially
    assert!(cb.is_trading_allowed());

    // Simulate -3% loss (OK)
    assert!(!cb.check_daily_loss(-0.03));
    assert!(cb.is_trading_allowed());

    // Simulate -5% loss (TRIGGER)
    assert!(cb.check_daily_loss(-0.05));
    assert!(!cb.is_trading_allowed());
}

#[test]
fn test_daily_loss_exact_boundary() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Exactly at -5% should trigger
    assert!(cb.check_daily_loss(-0.05));
    assert!(!cb.is_trading_allowed());
}

#[test]
fn test_consecutive_losses_trigger() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // 1st loss
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 1);
    assert!(cb.is_trading_allowed());

    // 2nd loss
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 2);
    assert!(cb.is_trading_allowed());

    // 3rd loss (TRIGGER)
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 3);
    assert!(!cb.is_trading_allowed());
}

#[test]
fn test_consecutive_losses_reset_on_win() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // 2 consecutive losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 2);

    // Win resets counter
    cb.record_trade_result(true);
    assert_eq!(cb.get_consecutive_losses(), 0);
    assert!(cb.is_trading_allowed());

    // Can lose again without triggering (fresh count)
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 2);
    assert!(cb.is_trading_allowed());
}

#[test]
fn test_volatility_detection() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05,
        max_consecutive_losses: 3,
        volatility_threshold: 2.0, // 2x average
    };
    let cb = CircuitBreakers::new(config);

    // Normal volatility (1.5x average)
    assert!(!cb.check_volatility(15.0, 10.0));

    // Exactly at threshold (2.0x)
    assert!(cb.check_volatility(20.0, 10.0));

    // High volatility (2.5x average)
    assert!(cb.check_volatility(25.0, 10.0));

    // Very high volatility (3.0x)
    assert!(cb.check_volatility(30.0, 10.0));
}

#[test]
fn test_volatility_with_zero_average() {
    let config = CircuitBreakerConfig::default();
    let cb = CircuitBreakers::new(config);

    // Should not panic or trigger with zero average
    assert!(!cb.check_volatility(100.0, 0.0));
}

#[test]
fn test_daily_reset_functionality() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Trigger circuit breaker with daily loss
    cb.check_daily_loss(-0.06);
    assert!(!cb.is_trading_allowed());
    assert_eq!(cb.get_daily_pnl(), -0.06);

    // Trigger with consecutive losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 3);

    // Reset daily
    cb.reset_daily();

    // Everything should be reset
    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_daily_pnl(), 0.0);
    assert_eq!(cb.get_consecutive_losses(), 0);
    assert!(!cb.is_triggered());
}

#[test]
fn test_force_reset() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Trigger multiple breakers
    cb.check_daily_loss(-0.10);
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    cb.record_trade_result(false);

    assert!(!cb.is_trading_allowed());

    // Force reset
    cb.force_reset();

    assert!(cb.is_trading_allowed());
    assert_eq!(cb.get_daily_pnl(), 0.0);
    assert_eq!(cb.get_consecutive_losses(), 0);
}

#[test]
fn test_multiple_triggers() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Trigger both daily loss AND consecutive losses
    cb.check_daily_loss(-0.06);
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    cb.record_trade_result(false);

    // Should be triggered
    assert!(!cb.is_trading_allowed());
    assert!(cb.is_triggered());
}

// ===== Integration Tests with TradingStrategy =====

fn create_test_strategy() -> TradingStrategy {
    let strategy_config = StrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        rsi_timeframe: "1H".to_string(),
        sentiment_threshold: 30,
    };

    let trading_config = TradingConfig {
        symbol: "FCPO".to_string(),
        risk_per_trade: 1.0,
        max_positions: 1,
        take_profit_percent: 2.0,
        stop_loss_percent: 1.5,
        max_daily_loss_percent: 5.0,
    };

    TradingStrategy::new(strategy_config, trading_config, 10000.0)
}

#[test]
fn test_strategy_integration_daily_loss() {
    let mut strategy = create_test_strategy();

    // Create and add position manually
    let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 5000.0, 1.0);
    strategy.add_position(position);

    // Close with big loss: 5000 -> 4700 = -300 per unit
    strategy.close_position("pos_1", 4700.0, CloseReason::StopLoss);

    // Daily P&L should be -300
    assert!(strategy.risk_state().daily_pnl < 0.0);

    // Now check if can open another position
    let result = strategy.can_open_position();
    // The -3% loss might not trigger -5% circuit breaker
    assert!(result.is_ok());
}

#[test]
fn test_strategy_integration_consecutive_losses() {
    let mut strategy = create_test_strategy();

    // Simulate 3 consecutive losing trades
    for i in 0..3 {
        let pos_id = format!("pos_{}", i);
        let pos = Position::new(&pos_id, "FCPO", OrderSide::Buy, 5000.0, 0.5);
        strategy.add_position(pos);

        // Close with small loss: 5000 -> 4950 = -50 per unit = -25 total
        strategy.close_position(&pos_id, 4950.0, CloseReason::StopLoss);

        // Wait before next trade
        if i < 2 {
            // After 2 losses, should still be allowed
            assert!(strategy.can_open_position().unwrap());
        }
    }

    // After 3 consecutive losses, circuit breaker should block
    assert!(!strategy.can_open_position().unwrap());
}

#[test]
fn test_strategy_integration_consecutive_reset_on_win() {
    let mut strategy = create_test_strategy();

    // 2 losing trades
    for i in 0..2 {
        let pos_id = format!("pos_loss_{}", i);
        let pos = Position::new(&pos_id, "FCPO", OrderSide::Buy, 5000.0, 0.5);
        strategy.add_position(pos);
        strategy.close_position(&pos_id, 4950.0, CloseReason::StopLoss);
    }

    // Consecutive losses = 2
    assert_eq!(strategy.risk_state().consecutive_losses, 2);

    // 1 winning trade
    let pos = Position::new("pos_win", "FCPO", OrderSide::Buy, 5000.0, 0.5);
    strategy.add_position(pos);
    strategy.close_position("pos_win", 5100.0, CloseReason::TakeProfit);

    // Consecutive losses should reset to 0
    assert_eq!(strategy.risk_state().consecutive_losses, 0);

    // Should be able to open position again
    assert!(strategy.can_open_position().unwrap());
}

#[test]
fn test_strategy_mixed_wins_and_losses() {
    let mut strategy = create_test_strategy();

    // Loss
    let pos1 = Position::new("pos_1", "FCPO", OrderSide::Buy, 5000.0, 0.5);
    strategy.add_position(pos1);
    strategy.close_position("pos_1", 4950.0, CloseReason::StopLoss);
    assert_eq!(strategy.risk_state().consecutive_losses, 1);

    // Win
    let pos2 = Position::new("pos_2", "FCPO", OrderSide::Buy, 5000.0, 0.5);
    strategy.add_position(pos2);
    strategy.close_position("pos_2", 5100.0, CloseReason::TakeProfit);
    assert_eq!(strategy.risk_state().consecutive_losses, 0);

    // Loss
    let pos3 = Position::new("pos_3", "FCPO", OrderSide::Buy, 5000.0, 0.5);
    strategy.add_position(pos3);
    strategy.close_position("pos_3", 4950.0, CloseReason::StopLoss);
    assert_eq!(strategy.risk_state().consecutive_losses, 1);

    // Win
    let pos4 = Position::new("pos_4", "FCPO", OrderSide::Buy, 5000.0, 0.5);
    strategy.add_position(pos4);
    strategy.close_position("pos_4", 5100.0, CloseReason::TakeProfit);
    assert_eq!(strategy.risk_state().consecutive_losses, 0);

    // Should still be able to trade
    assert!(strategy.can_open_position().unwrap());
}

#[test]
fn test_circuit_breaker_custom_config() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.03, // Stricter: -3%
        max_consecutive_losses: 2, // Only 2 losses allowed
        volatility_threshold: 1.5, // Lower threshold
    };
    let mut cb = CircuitBreakers::new(config);

    // -3% triggers (stricter)
    assert!(cb.check_daily_loss(-0.03));
    assert!(!cb.is_trading_allowed());

    // Reset
    cb.reset_daily();

    // 2 losses trigger (stricter)
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert!(!cb.is_trading_allowed());

    // Reset
    cb.reset_daily();

    // 1.6x volatility triggers (stricter, > 1.5 threshold)
    assert!(cb.check_volatility(16.0, 10.0));
}

#[test]
fn test_circuit_breaker_permissive_config() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.10, // More permissive: -10%
        max_consecutive_losses: 5, // Allow 5 losses
        volatility_threshold: 3.0, // Higher threshold
    };
    let mut cb = CircuitBreakers::new(config);

    // -8% doesn't trigger
    assert!(!cb.check_daily_loss(-0.08));
    assert!(cb.is_trading_allowed());

    // 4 losses don't trigger
    for _ in 0..4 {
        cb.record_trade_result(false);
    }
    assert_eq!(cb.get_consecutive_losses(), 4);
    assert!(cb.is_trading_allowed());

    // 2.5x volatility doesn't trigger
    assert!(!cb.check_volatility(25.0, 10.0));
}

#[test]
fn test_state_preservation_across_operations() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Set daily P&L
    cb.check_daily_loss(-0.03);
    assert_eq!(cb.get_daily_pnl(), -0.03);

    // Add consecutive losses
    cb.record_trade_result(false);
    assert_eq!(cb.get_consecutive_losses(), 1);

    // Check volatility (doesn't affect state)
    cb.check_volatility(15.0, 10.0);

    // State should be preserved
    assert_eq!(cb.get_daily_pnl(), -0.03);
    assert_eq!(cb.get_consecutive_losses(), 1);
}

#[test]
fn test_circuit_breaker_after_partial_recovery() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Big loss
    cb.check_daily_loss(-0.06);
    assert!(!cb.is_trading_allowed());

    // Can't trade even if we theoretically recover
    // (need manual reset or new day)
    cb.check_daily_loss(-0.02); // "Recovered" to -2%
    assert!(!cb.is_trading_allowed()); // Still blocked

    // Need explicit reset
    cb.force_reset();
    assert!(cb.is_trading_allowed());
}
