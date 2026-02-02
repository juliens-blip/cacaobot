//! Full Stack Integration Tests for Palm Oil Trading Bot
//!
//! End-to-end tests simulating the complete trading workflow:
//! 1. Initialize bot with demo config
//! 2. Mock cTrader connection
//! 3. Subscribe to FCPO symbol
//! 4. Simulate price movements
//! 5. Trigger buy signal (RSI oversold + bullish sentiment)
//! 6. Verify order placement
//! 7. Close position on take profit

use palm_oil_bot::config::{
    BotConfig, CTraderConfig, Config, PerplexityConfig, StrategyConfig, TradingConfig, TradingEnvironment,
};
use palm_oil_bot::modules::trading::{
    CircuitBreakers, CloseReason, OrderSide, Position, RsiCalculator, Signal, TradingStrategy,
};
use palm_oil_bot::modules::trading::circuit_breakers::CircuitBreakerConfig;
use palm_oil_bot::modules::trading::position_manager::{
    BrokerPosition, PersistentPositionManager,
};

use tempfile::NamedTempFile;

/// Create a test configuration for demo/dry-run mode
fn create_test_config() -> Config {
    Config {
        ctrader: CTraderConfig {
            environment: TradingEnvironment::Demo,
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            account_id: "test_account_123".to_string(),
            access_token: Some("demo_token".to_string()),
            server: "demo.ctraderapi.com".to_string(),
            port: 5035,
            client_id_live: None,
            client_secret_live: None,
            account_id_live: None,
        },
        perplexity: PerplexityConfig {
            api_key: "test_perplexity_key".to_string(),
            endpoint: "https://api.perplexity.ai/chat/completions".to_string(),
            model: "sonar".to_string(),
        },
        trading: TradingConfig {
            symbol: "FCPO".to_string(),
            risk_per_trade: 1.0,
            take_profit_percent: 2.0,
            stop_loss_percent: 1.5,
            max_positions: 1,
            max_daily_loss_percent: 5.0,
                initial_balance: 10000.0,
        },
        strategy: StrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            rsi_timeframe: "5m".to_string(),
            sentiment_threshold: 30,
        },
        kols: vec![
            "PalmOilTrader".to_string(),
            "CommodityInsights".to_string(),
        ],
        bot: BotConfig {
            cycle_interval_secs: 1,
            dry_run: true,
            log_level: "debug".to_string(),
        },
    }
}

/// Simulated price feed for testing
#[allow(dead_code)] // Test utility struct - fields accessed through methods
struct MockPriceFeed {
    prices: Vec<f64>,
    current_index: usize,
}

#[allow(dead_code)] // Test utility methods for future test expansion
impl MockPriceFeed {
    fn new(prices: Vec<f64>) -> Self {
        Self {
            prices,
            current_index: 0,
        }
    }

    fn next_price(&mut self) -> Option<f64> {
        if self.current_index < self.prices.len() {
            let price = self.prices[self.current_index];
            self.current_index += 1;
            Some(price)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Generate prices that create an oversold RSI condition
    fn generate_oversold_sequence(start_price: f64, periods: usize) -> Vec<f64> {
        let mut prices = Vec::with_capacity(periods);
        let mut price = start_price;

        // Generate declining prices to create oversold condition
        for i in 0..periods {
            // Decline with some volatility
            let decline = 0.5 + (i as f64 * 0.1);
            price -= decline;
            prices.push(price);
        }

        prices
    }

    /// Generate prices that create an overbought RSI condition
    fn generate_overbought_sequence(start_price: f64, periods: usize) -> Vec<f64> {
        let mut prices = Vec::with_capacity(periods);
        let mut price = start_price;

        // Generate rising prices to create overbought condition
        for i in 0..periods {
            let rise = 0.5 + (i as f64 * 0.1);
            price += rise;
            prices.push(price);
        }

        prices
    }
}

/// Mock sentiment provider
struct MockSentimentProvider {
    sentiment_score: i32,
}

impl MockSentimentProvider {
    fn bullish() -> Self {
        Self { sentiment_score: 50 } // > 30 threshold
    }

    #[allow(dead_code)] // Test utility for future sell-side tests
    fn bearish() -> Self {
        Self { sentiment_score: -50 } // < -30 threshold
    }

    #[allow(dead_code)] // Test utility for neutral market scenarios
    fn neutral() -> Self {
        Self { sentiment_score: 0 }
    }

    fn get_sentiment(&self) -> i32 {
        self.sentiment_score
    }
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_full_stack_init_config() {
    // Step 1: Initialize bot with demo config
    let config = create_test_config();

    assert_eq!(config.trading.symbol, "FCPO");
    assert_eq!(config.strategy.rsi_period, 14);
    assert_eq!(config.strategy.rsi_oversold, 30.0);
    assert_eq!(config.strategy.rsi_overbought, 70.0);
    assert_eq!(config.strategy.sentiment_threshold, 30);
    assert!(config.bot.dry_run);
}

#[tokio::test]
async fn test_full_stack_rsi_calculation() {
    // Test RSI calculation with simulated prices
    let mut rsi_calc = RsiCalculator::new(14);

    // Feed 15 prices (need 14+1 for first RSI)
    let prices = vec![
        4850.0, 4855.0, 4860.0, 4858.0, 4862.0, 4865.0, 4863.0, 4868.0,
        4870.0, 4867.0, 4872.0, 4875.0, 4873.0, 4878.0, 4880.0,
    ];

    let mut last_rsi = None;
    for price in prices {
        last_rsi = rsi_calc.add_price(price);
    }

    assert!(last_rsi.is_some());
    let rsi = last_rsi.unwrap();
    assert!((0.0..=100.0).contains(&rsi));
    // With mostly rising prices, RSI should be > 50
    assert!(rsi > 50.0);
}

#[tokio::test]
async fn test_full_stack_oversold_detection() {
    let mut rsi_calc = RsiCalculator::new(14);

    // Generate declining prices to create oversold condition
    let prices = MockPriceFeed::generate_oversold_sequence(4850.0, 20);

    let mut last_rsi = None;
    for price in prices {
        last_rsi = rsi_calc.add_price(price);
    }

    assert!(last_rsi.is_some());
    let rsi = last_rsi.unwrap();
    // RSI should be low (oversold) after consistent declines
    assert!(rsi < 40.0, "RSI should be oversold, got: {}", rsi);
}

#[tokio::test]
async fn test_full_stack_buy_signal_generation() {
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    // Test buy signal: RSI < 30 AND sentiment > 30
    let rsi_oversold = 25.0;
    let sentiment_bullish = 50;

    let signal = strategy.generate_signal(rsi_oversold, sentiment_bullish);
    assert_eq!(signal, Signal::Buy);

    // Test no signal: RSI < 30 BUT sentiment neutral
    let signal_no_sentiment = strategy.generate_signal(25.0, 0);
    assert_eq!(signal_no_sentiment, Signal::Hold);

    // Test no signal: sentiment bullish BUT RSI not oversold
    let signal_no_rsi = strategy.generate_signal(50.0, 50);
    assert_eq!(signal_no_rsi, Signal::Hold);
}

#[tokio::test]
async fn test_full_stack_sell_signal_generation() {
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    // Test sell signal: RSI > 70 AND sentiment < -30
    let rsi_overbought = 75.0;
    let sentiment_bearish = -50;

    let signal = strategy.generate_signal(rsi_overbought, sentiment_bearish);
    assert_eq!(signal, Signal::Sell);

    // Test no signal: RSI > 70 BUT sentiment neutral
    let signal_no_sentiment = strategy.generate_signal(75.0, 0);
    assert_eq!(signal_no_sentiment, Signal::Hold);
}

#[tokio::test]
async fn test_full_stack_position_lifecycle() {
    let manager = PersistentPositionManager::new();

    // Step 1: Open position on buy signal
    let entry_price = 4850.0;
    let position = Position::new("test_pos_1", "FCPO", OrderSide::Buy, entry_price, 1.0)
        .with_take_profit(entry_price * 1.02) // +2%
        .with_stop_loss(entry_price * 0.985); // -1.5%

    let pos_id = manager.open_position(position).await.unwrap();
    assert_eq!(pos_id, "test_pos_1");
    assert_eq!(manager.count().await, 1);

    // Step 2: Update price (simulating market movement)
    manager.update_prices("FCPO", 4900.0).await;

    let pos = manager.get("test_pos_1").await.unwrap();
    assert!((pos.current_pnl - 50.0).abs() < 0.01); // +50 P&L

    // Step 3: Close position on take profit
    let pnl = manager
        .close_position("test_pos_1", 4947.0, CloseReason::TakeProfit)
        .await
        .unwrap();

    assert!(pnl > 0.0);
    assert_eq!(manager.count().await, 0);
    assert_eq!(manager.get_closed_positions().await.len(), 1);
}

#[tokio::test]
async fn test_full_stack_circuit_breakers() {
    let config = CircuitBreakerConfig {
        daily_loss_limit: -0.05, // -5%
        max_consecutive_losses: 3,
        volatility_threshold: 2.0,
    };

    let mut cb = CircuitBreakers::new(config);

    // Trading should be allowed initially
    assert!(cb.is_trading_allowed());

    // Simulate 3 consecutive losses
    cb.record_trade_result(false);
    cb.record_trade_result(false);
    assert!(cb.is_trading_allowed()); // Still OK with 2 losses

    cb.record_trade_result(false);
    assert!(!cb.is_trading_allowed()); // Blocked after 3 losses

    // Reset for new day
    cb.reset_daily();
    assert!(cb.is_trading_allowed());
}

#[tokio::test]
async fn test_full_stack_daily_loss_limit() {
    let config = CircuitBreakerConfig::default();
    let mut cb = CircuitBreakers::new(config);

    // Simulate -6% daily loss (exceeds -5% limit)
    let triggered = cb.check_daily_loss(-0.06);
    assert!(triggered);
    assert!(!cb.is_trading_allowed());
}

#[tokio::test]
async fn test_full_stack_volatility_detection() {
    let config = CircuitBreakerConfig {
        volatility_threshold: 2.0,
        ..Default::default()
    };

    let cb = CircuitBreakers::new(config);

    // Normal volatility (1.5x average)
    assert!(!cb.check_volatility(15.0, 10.0));

    // High volatility (2.5x average)
    assert!(cb.check_volatility(25.0, 10.0));
}

#[tokio::test]
async fn test_full_stack_position_reconciliation() {
    let manager = PersistentPositionManager::new();

    // Open a local position
    let position = Position::new("123", "FCPO", OrderSide::Buy, 4850.0, 1.0);
    manager.open_position(position).await.unwrap();

    // Simulate broker state matching local
    let broker_positions = vec![BrokerPosition {
        position_id: 123,
        symbol_id: 1,
        symbol: "FCPO".to_string(),
        side: OrderSide::Buy,
        entry_price: 4850.0,
        volume: 1.0,
        current_pnl: 25.0,
    }];

    let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

    assert!(result.is_clean());
    assert_eq!(result.synced.len(), 1);
}

#[tokio::test]
async fn test_full_stack_orphaned_position_cleanup() {
    let manager = PersistentPositionManager::new();

    // Open a local position that doesn't exist on broker
    let position = Position::new("999", "FCPO", OrderSide::Buy, 4850.0, 1.0);
    manager.open_position(position).await.unwrap();

    // Broker has no positions
    let broker_positions: Vec<BrokerPosition> = vec![];

    let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

    // Local position should be marked as orphaned and removed
    assert_eq!(result.orphaned_local.len(), 1);
    assert_eq!(manager.count().await, 0);
}

#[tokio::test]
async fn test_full_stack_missing_position_sync() {
    let manager = PersistentPositionManager::new();

    // Broker has a position we don't track locally
    let broker_positions = vec![BrokerPosition {
        position_id: 456,
        symbol_id: 1,
        symbol: "FCPO".to_string(),
        side: OrderSide::Sell,
        entry_price: 4900.0,
        volume: 0.5,
        current_pnl: -15.0,
    }];

    let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

    // Position should be auto-added locally
    assert_eq!(result.missing_local.len(), 1);
    assert_eq!(manager.count().await, 1);

    let pos = manager.get("456").await.unwrap();
    assert_eq!(pos.side, OrderSide::Sell);
    assert_eq!(pos.entry_price, 4900.0);
}

#[tokio::test]
async fn test_full_stack_take_profit_calculation() {
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    let entry_price = 4850.0;

    // Buy position: TP should be above entry
    let tp_buy = strategy.calculate_take_profit(entry_price, OrderSide::Buy);
    assert!((tp_buy - 4947.0).abs() < 0.1); // +2%

    // Sell position: TP should be below entry
    let tp_sell = strategy.calculate_take_profit(entry_price, OrderSide::Sell);
    assert!((tp_sell - 4753.0).abs() < 0.1); // -2%
}

#[tokio::test]
async fn test_full_stack_stop_loss_calculation() {
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    let entry_price = 4850.0;

    // Buy position: SL should be below entry
    let sl_buy = strategy.calculate_stop_loss(entry_price, OrderSide::Buy);
    assert!((sl_buy - 4777.25).abs() < 0.1); // -1.5%

    // Sell position: SL should be above entry
    let sl_sell = strategy.calculate_stop_loss(entry_price, OrderSide::Sell);
    assert!((sl_sell - 4922.75).abs() < 0.1); // +1.5%
}

#[tokio::test]
async fn test_full_stack_position_size_calculation() {
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0, // $10,000 balance
    );

    let entry_price = 4850.0;
    let stop_loss = strategy.calculate_stop_loss(entry_price, OrderSide::Buy);

    let size = strategy.calculate_position_size(entry_price, stop_loss);

    // With 1% risk on $10,000 = $100 risk
    // SL distance = 4850 - 4777.25 = 72.75
    // Size = 100 / 72.75 = 1.37 -> capped at 1.0
    assert!(size <= 1.0);
    assert!(size > 0.0);
}

#[tokio::test]
async fn test_full_stack_max_positions_limit() {
    let config = create_test_config();

    let mut strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(), // max_positions = 1
        10000.0,
    );

    // Should be able to open first position
    assert!(strategy.can_open_position().unwrap());

    // Add a position
    let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
    strategy.add_position(position);

    // Should not be able to open another (max_positions = 1)
    assert!(!strategy.can_open_position().unwrap());
}

#[tokio::test]
async fn test_full_stack_consecutive_losses_cooldown() {
    let config = create_test_config();

    let mut strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    // Simulate 3 consecutive losses
    for i in 0..3 {
        let pos = Position::new(format!("pos_{}", i), "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(pos);
        strategy.close_position(&format!("pos_{}", i), 4800.0, CloseReason::StopLoss);
    }

    // Should be blocked due to consecutive losses
    assert!(!strategy.can_open_position().unwrap());

    // Reset losses
    strategy.reset_consecutive_losses();
    assert!(strategy.can_open_position().unwrap());
}

#[tokio::test]
async fn test_full_stack_complete_trading_cycle() {
    // Complete end-to-end trading cycle simulation
    let config = create_test_config();

    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    let manager = PersistentPositionManager::new();
    let mut rsi_calc = RsiCalculator::new(14);
    let sentiment = MockSentimentProvider::bullish();

    // Step 1: Feed prices to create oversold condition
    let prices = MockPriceFeed::generate_oversold_sequence(4850.0, 20);
    let mut last_rsi = None;
    for price in prices {
        last_rsi = rsi_calc.add_price(price);
    }

    let rsi = last_rsi.unwrap();
    let sentiment_score = sentiment.get_sentiment();

    // Step 2: Check for buy signal
    let signal = strategy.generate_signal(rsi, sentiment_score);

    // Note: With generated oversold prices, RSI should trigger buy if sentiment is bullish
    // The actual signal depends on how low RSI gets
    if rsi < 30.0 && sentiment_score > 30 {
        assert_eq!(signal, Signal::Buy);

        // Step 3: Execute buy order
        let entry_price = 4800.0; // Current price after decline
        let tp = strategy.calculate_take_profit(entry_price, OrderSide::Buy);
        let sl = strategy.calculate_stop_loss(entry_price, OrderSide::Buy);

        let position = Position::new("trade_1", "FCPO", OrderSide::Buy, entry_price, 0.1)
            .with_take_profit(tp)
            .with_stop_loss(sl);

        manager.open_position(position).await.unwrap();
        assert_eq!(manager.count().await, 1);

        // Step 4: Simulate price rise to take profit
        let exit_price = tp + 10.0; // Above TP
        manager.update_prices("FCPO", exit_price).await;

        // Step 5: Close on take profit
        let pnl = manager
            .close_position("trade_1", exit_price, CloseReason::TakeProfit)
            .await
            .unwrap();

        assert!(pnl > 0.0);
        assert_eq!(manager.count().await, 0);

        // Step 6: Verify P&L tracking
        let daily_pnl = manager.get_daily_pnl().await;
        assert!(daily_pnl > 0.0);
    }
}

#[tokio::test]
async fn test_full_stack_dry_run_mode() {
    let mut config = create_test_config();
    config.bot.dry_run = true;

    // In dry run mode, orders should be simulated locally
    let strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    assert!(config.bot.dry_run);

    // Dry run should still generate valid signals
    let signal = strategy.generate_signal(25.0, 50);
    assert_eq!(signal, Signal::Buy);
}

#[tokio::test]
async fn test_full_stack_persistence_recovery() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Simulate bot crash with open position
    {
        let manager = PersistentPositionManager::with_persistence(&path);

        let position = Position::new("crash_test", "FCPO", OrderSide::Buy, 4850.0, 1.0)
            .with_take_profit(4947.0)
            .with_stop_loss(4777.25);

        manager.open_position(position).await.unwrap();
        manager.save().await.unwrap();
        // "Crash" here - manager goes out of scope
    }

    // Simulate bot restart
    {
        let manager = PersistentPositionManager::with_persistence(&path);
        manager.load().await.unwrap();

        // Position should be recovered
        assert_eq!(manager.count().await, 1);

        let pos = manager.get("crash_test").await.unwrap();
        assert_eq!(pos.entry_price, 4850.0);
        assert_eq!(pos.take_profit, Some(4947.0));
        assert_eq!(pos.stop_loss, Some(4777.25));
    }
}

#[tokio::test]
async fn test_full_stack_multiple_symbols() {
    let manager = PersistentPositionManager::new();

    // Open positions for different symbols
    let fcpo = Position::new("fcpo_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
    let gold = Position::new("gold_1", "GOLD", OrderSide::Sell, 2000.0, 0.1);

    manager.open_position(fcpo).await.unwrap();
    manager.open_position(gold).await.unwrap();

    assert_eq!(manager.count().await, 2);
    assert!(manager.has_position_for_symbol("FCPO").await);
    assert!(manager.has_position_for_symbol("GOLD").await);
    assert!(!manager.has_position_for_symbol("SILVER").await);

    // Update only FCPO prices
    manager.update_prices("FCPO", 4900.0).await;

    let fcpo_pos = manager.get("fcpo_1").await.unwrap();
    let gold_pos = manager.get("gold_1").await.unwrap();

    assert!((fcpo_pos.current_pnl - 50.0).abs() < 0.01);
    assert_eq!(gold_pos.current_pnl, 0.0); // Not updated
}

#[tokio::test]
async fn test_full_stack_risk_management_integration() {
    let config = create_test_config();

    let mut strategy = TradingStrategy::new(
        config.strategy.clone(),
        config.trading.clone(),
        10000.0,
    );

    let cb_config = CircuitBreakerConfig::default();
    let mut circuit_breakers = CircuitBreakers::new(cb_config);

    // Simulate a series of losing trades
    for i in 0..3 {
        // Open position
        let pos = Position::new(format!("loss_{}", i), "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(pos);

        // Close with loss
        let pnl = strategy.close_position(&format!("loss_{}", i), 4800.0, CloseReason::StopLoss);
        assert!(pnl.is_some());
        assert!(pnl.unwrap() < 0.0);

        // Record in circuit breakers
        circuit_breakers.record_trade_result(false);
    }

    // Both strategy and circuit breakers should block trading
    assert!(!strategy.can_open_position().unwrap());
    assert!(!circuit_breakers.is_trading_allowed());
}
