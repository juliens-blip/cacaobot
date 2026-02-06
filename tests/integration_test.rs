//! Integration tests for Palm Oil Trading Bot
//!
//! Tests the complete trading workflow: RSI calculation → sentiment → signal generation → position management

use palm_oil_bot::config::Config;
use palm_oil_bot::modules::trading::{
    indicators::RsiCalculator,
    orders::OrderSide,
    strategy::{Signal, TradingStrategy},
};

#[test]
fn test_complete_buy_signal_workflow() {
    let config = Config::default();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    let mut rsi_calc = RsiCalculator::new(14);
    
    // Generate 15 prices to have RSI ready (14-period + 1)
    let prices = vec![
        4850.0, 4840.0, 4830.0, 4820.0, 4810.0, // Declining
        4800.0, 4790.0, 4780.0, 4770.0, 4760.0, // Continuing decline
        4750.0, 4740.0, 4730.0, 4720.0, 4710.0, // Heavy sell-off
    ];
    
    let mut rsi = None;
    for price in prices {
        rsi = rsi_calc.add_price(price);
    }
    
    let rsi_value = rsi.expect("RSI should be calculated");
    
    // After heavy decline, RSI should be oversold
    assert!(rsi_value < 30.0, "RSI should be oversold: {}", rsi_value);
    
    // Simulate bullish sentiment
    let sentiment = 50;
    
    // Should generate BUY signal
    let signal = strategy.generate_signal(rsi_value, sentiment);
    assert_eq!(signal, Signal::Buy, "Should generate BUY signal");
    
    // Should be able to open position
    assert!(strategy.can_open_position().unwrap(), "Should allow opening position");
    
    // Calculate position size
    let entry_price = 4710.0;
    let sl_price = strategy.calculate_stop_loss(entry_price, OrderSide::Buy);
    let position_size = strategy.calculate_position_size(entry_price, sl_price);
    
    assert!(position_size > 0.0, "Position size should be positive");
    // position_size is in base currency units (risk_amount / risk_per_unit)
    let expected = 10000.0 * 0.01 / (entry_price - sl_price);
    assert!((position_size - expected).abs() < 0.1, "Position size should match risk calc");
}

#[test]
fn test_complete_sell_signal_workflow() {
    let config = Config::default();
    let strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    let mut rsi_calc = RsiCalculator::new(14);
    
    // Generate prices with strong uptrend
    let prices = vec![
        4700.0, 4710.0, 4720.0, 4730.0, 4740.0, // Rising
        4750.0, 4760.0, 4770.0, 4780.0, 4790.0, // Continuing rise
        4800.0, 4810.0, 4820.0, 4830.0, 4840.0, // Strong rally
    ];
    
    let mut rsi = None;
    for price in prices {
        rsi = rsi_calc.add_price(price);
    }
    
    let rsi_value = rsi.expect("RSI should be calculated");
    
    // After strong rally, RSI should be overbought
    assert!(rsi_value > 70.0, "RSI should be overbought: {}", rsi_value);
    
    // Simulate bearish sentiment
    let sentiment = -50;
    
    // Should generate SELL signal
    let signal = strategy.generate_signal(rsi_value, sentiment);
    assert_eq!(signal, Signal::Sell, "Should generate SELL signal");
}

#[test]
fn test_position_lifecycle_with_take_profit() {
    let config = Config::default();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    // Open a BUY position
    let entry_price = 4800.0;
    let volume = 1.0;
    
    let position = palm_oil_bot::modules::trading::orders::Position::new(
        "test_pos_1",
        "FCPO",
        OrderSide::Buy,
        entry_price,
        volume,
    );
    
    strategy.add_position(position.clone());
    
    // Verify position is open
    assert_eq!(strategy.get_open_positions().len(), 1);
    
    // Calculate TP price (+2%)
    let tp_price = entry_price * 1.02;
    
    // Check if TP is hit
    let exit_reason = strategy.check_position_exit(&position, tp_price);
    assert!(exit_reason.is_some(), "Should trigger TP exit");
    
    // Close position
    let pnl = strategy.close_position(
        "test_pos_1",
        tp_price,
        palm_oil_bot::modules::trading::orders::CloseReason::TakeProfit,
    );
    
    assert!(pnl.is_some(), "Should return P&L");
    assert!(pnl.unwrap() > 0.0, "P&L should be positive");
    
    // Verify position is closed
    assert_eq!(strategy.get_open_positions().len(), 0);
}

#[test]
fn test_position_lifecycle_with_stop_loss() {
    let config = Config::default();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    // Open a BUY position
    let entry_price = 4800.0;
    let volume = 1.0;
    
    let position = palm_oil_bot::modules::trading::orders::Position::new(
        "test_pos_2",
        "FCPO",
        OrderSide::Buy,
        entry_price,
        volume,
    );
    
    strategy.add_position(position.clone());
    
    // Calculate SL price (-1.5%)
    let sl_price = entry_price * 0.985;
    
    // Check if SL is hit
    let exit_reason = strategy.check_position_exit(&position, sl_price);
    assert!(exit_reason.is_some(), "Should trigger SL exit");
    
    // Close position
    let pnl = strategy.close_position(
        "test_pos_2",
        sl_price,
        palm_oil_bot::modules::trading::orders::CloseReason::StopLoss,
    );
    
    assert!(pnl.is_some(), "Should return P&L");
    assert!(pnl.unwrap() < 0.0, "P&L should be negative");
}

#[test]
fn test_risk_management_max_positions() {
    let config = Config::default();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    // Open max positions (1)
    let position = palm_oil_bot::modules::trading::orders::Position::new(
        "test_pos_3",
        "FCPO",
        OrderSide::Buy,
        4800.0,
        1.0,
    );
    
    strategy.add_position(position);
    
    // Try to open another position
    let can_open = strategy.can_open_position().unwrap();
    assert!(!can_open, "Should not allow opening more positions");
}

#[test]
fn test_risk_management_circuit_breaker() {
    let config = Config::default();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        config.trading,
        10000.0,
    );
    
    // Simulate multiple losing trades
    for i in 0..5 {
        let position = palm_oil_bot::modules::trading::orders::Position::new(
            format!("loss_{}", i),
            "FCPO",
            OrderSide::Buy,
            4800.0,
            1.0,
        );
        
        strategy.add_position(position);
        
        // Close with loss
        strategy.close_position(
            &format!("loss_{}", i),
            4750.0, // -50 loss
            palm_oil_bot::modules::trading::orders::CloseReason::StopLoss,
        );
    }
    
    // Total loss: -250 on 10000 balance = -2.5%
    // Circuit breaker at -5% should not trigger yet
    let can_open = strategy.can_open_position();
    assert!(can_open.is_ok());
    
    // Add more losses to trigger circuit breaker
    for i in 5..12 {
        let position = palm_oil_bot::modules::trading::orders::Position::new(
            format!("loss_{}", i),
            "FCPO",
            OrderSide::Buy,
            4800.0,
            1.0,
        );
        
        strategy.add_position(position);
        strategy.close_position(
            &format!("loss_{}", i),
            4750.0,
            palm_oil_bot::modules::trading::orders::CloseReason::StopLoss,
        );
    }
    
    // Total loss should exceed -5% now
    let can_open = strategy.can_open_position().unwrap();
    assert!(!can_open, "Circuit breaker should prevent opening positions");
}

#[test]
fn test_rsi_calculation_accuracy() {
    let mut rsi_calc = RsiCalculator::new(14);
    
    // Known price series
    let prices = vec![
        44.0, 44.25, 44.5, 43.75, 44.5,
        44.25, 44.0, 43.5, 44.0, 44.5,
        44.75, 45.0, 45.5, 45.25, 45.75,
    ];
    
    let mut rsi = None;
    for price in prices {
        rsi = rsi_calc.add_price(price);
    }
    
    let rsi_value = rsi.expect("RSI should be calculated");
    
    // RSI should be between 0 and 100
    assert!((0.0..=100.0).contains(&rsi_value), "RSI out of range: {}", rsi_value);
    
    // For this upward trending series, RSI should be > 50
    assert!(rsi_value > 50.0, "RSI should indicate upward trend: {}", rsi_value);
}

#[test]
fn test_metrics_tracking() {
    use palm_oil_bot::modules::monitoring::{MetricsHandle, Trade};
    
    let metrics = MetricsHandle::new(10000.0);
    
    // Add some trades
    metrics.with_metrics_mut(|m| {
        m.add_trade(Trade::new("trade_1".into(), "BUY".into(), 1.0, 4800.0));
        m.close_trade("trade_1", 4896.0); // +2% profit
        
        m.add_trade(Trade::new("trade_2".into(), "SELL".into(), 1.0, 4900.0));
        m.close_trade("trade_2", 4975.0); // -1.5% loss
    });
    
    // Verify metrics
    metrics.with_metrics(|m| {
        assert_eq!(m.get_total_trades(), 2, "Should have 2 trades");
        
        let win_rate = m.calculate_win_rate();
        assert_eq!(win_rate, 50.0, "Win rate should be 50%");
        
        let open_positions = m.get_open_positions();
        assert_eq!(open_positions.len(), 0, "No positions should be open");
    });
}

#[test]
fn test_sentiment_parsing() {
    use palm_oil_bot::modules::scraper::sentiment::SentimentAnalyzer;
    
    let analyzer = SentimentAnalyzer::new();
    
    // Test positive sentiment text
    let positive_text = "FCPO palm oil futures surge on strong demand. Bullish outlook prevails. Score: +75";
    let sentiment = analyzer.parse_sentiment(positive_text);
    assert!(sentiment > 0, "Should detect positive sentiment");
    
    // Test negative sentiment text
    let negative_text = "Palm oil market crashes amid oversupply concerns. Bearish trend continues. Score: -60";
    let sentiment = analyzer.parse_sentiment(negative_text);
    assert!(sentiment < 0, "Should detect negative sentiment");
}
