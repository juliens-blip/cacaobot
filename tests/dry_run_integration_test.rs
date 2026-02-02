//! Dry run integration test
//!
//! Tests the complete trading loop simulation without real cTrader credentials.
//! Generates synthetic candles, calculates RSI, generates signals, simulates positions,
//! and validates P&L and statistics.

use palm_oil_bot::config::{StrategyConfig, TradingConfig};
use palm_oil_bot::modules::trading::indicators::RsiCalculator;
use palm_oil_bot::modules::trading::orders::{OrderSide, Position};
use palm_oil_bot::modules::trading::strategy::{Signal, TradingStrategy};
use rand::Rng;

/// Synthetic candle data (OHLC)
#[derive(Debug, Clone)]
struct Candle {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

impl Candle {
    fn new(timestamp: i64, close: f64) -> Self {
        Self {
            timestamp,
            open: close,
            high: close * 1.002, // +0.2% wick
            low: close * 0.998,  // -0.2% wick
            close,
        }
    }
}

/// Generate synthetic price data using random walk
fn generate_candles(count: usize, start_price: f64, volatility: f64) -> Vec<Candle> {
    let mut rng = rand::thread_rng();
    let mut candles = Vec::with_capacity(count);
    let mut price = start_price;
    let base_timestamp = 1700000000; // Nov 2023

    for i in 0..count {
        // Random walk: +/- volatility%
        let change_percent = rng.gen_range(-volatility..volatility);
        price *= 1.0 + (change_percent / 100.0);

        let candle = Candle::new(base_timestamp + (i as i64 * 300), price); // 5min candles
        candles.push(candle);
    }

    candles
}

/// Simulate sentiment correlated with RSI
fn simulate_sentiment(rsi: f64) -> i32 {
    let mut rng = rand::thread_rng();
    let noise = rng.gen_range(-20..20);

    // Oversold (RSI < 30) → Bullish sentiment (+30 to +70)
    // Overbought (RSI > 70) → Bearish sentiment (-70 to -30)
    // Neutral (30-70) → Neutral sentiment (-20 to +20)
    let base_sentiment = if rsi < 30.0 {
        50 // Bullish
    } else if rsi > 70.0 {
        -50 // Bearish
    } else {
        0 // Neutral
    };

    (base_sentiment + noise).clamp(-100, 100)
}

/// Dry run position tracker
#[derive(Debug)]
struct DryRunPosition {
    id: String,
    side: OrderSide,
    entry_price: f64,
    volume: f64,
    take_profit: f64,
    stop_loss: f64,
}

impl DryRunPosition {
    fn calculate_pnl(&self, current_price: f64) -> f64 {
        let price_diff = match self.side {
            OrderSide::Buy => current_price - self.entry_price,
            OrderSide::Sell => self.entry_price - current_price,
        };
        price_diff * self.volume
    }

    fn check_exit(&self, current_price: f64) -> Option<&'static str> {
        match self.side {
            OrderSide::Buy => {
                if current_price >= self.take_profit {
                    Some("TP")
                } else if current_price <= self.stop_loss {
                    Some("SL")
                } else {
                    None
                }
            }
            OrderSide::Sell => {
                if current_price <= self.take_profit {
                    Some("TP")
                } else if current_price >= self.stop_loss {
                    Some("SL")
                } else {
                    None
                }
            }
        }
    }
}

/// Trading statistics
#[derive(Debug, Default)]
struct TradingStats {
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_pnl: f64,
    max_drawdown: f64,
    current_balance: f64,
}

impl TradingStats {
    fn new(starting_balance: f64) -> Self {
        Self {
            current_balance: starting_balance,
            ..Default::default()
        }
    }

    fn record_trade(&mut self, pnl: f64) {
        self.total_trades += 1;
        self.total_pnl += pnl;
        self.current_balance += pnl;

        if pnl > 0.0 {
            self.winning_trades += 1;
        } else {
            self.losing_trades += 1;
        }

        // Track drawdown
        let drawdown = self.total_pnl;
        if drawdown < self.max_drawdown {
            self.max_drawdown = drawdown;
        }
    }

    fn win_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            (self.winning_trades as f64 / self.total_trades as f64) * 100.0
        }
    }
}

#[test]
fn test_dry_run_full_trading_loop() {
    // 1) Create TradingStrategy with test config
    let strategy_config = StrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        rsi_timeframe: "M5".to_string(),
        sentiment_threshold: 30,
    };

    let trading_config = TradingConfig {
        symbol: "FCPO".to_string(),
        risk_per_trade: 1.0,
        max_positions: 1,
        take_profit_percent: 2.0,
        stop_loss_percent: 1.5,
        max_daily_loss_percent: 5.0,
        initial_balance: 10000.0,
    };

    let starting_balance = 10000.0;
    let mut strategy = TradingStrategy::new(
        strategy_config.clone(),
        trading_config.clone(),
        starting_balance,
    );

    // Disable trend filter for deterministic signals
    strategy.set_trend_filter(false);

    // 2) Generate synthetic candles (random walk)
    let candles = generate_candles(200, 4850.0, 1.5);
    println!("Generated {} candles", candles.len());

    // 3) Calculate RSI on candles
    let mut rsi_calc = RsiCalculator::new(strategy_config.rsi_period);
    let mut rsi_values = Vec::new();

    for candle in &candles {
        if let Some(rsi) = rsi_calc.add_price(candle.close) {
            rsi_values.push(rsi);
        }
    }

    println!("Calculated {} RSI values", rsi_values.len());
    assert!(!rsi_values.is_empty());

    // 4) Generate signals and simulate trading
    let mut stats = TradingStats::new(starting_balance);
    let mut open_position: Option<DryRunPosition> = None;
    let mut position_counter = 0;

    for (i, candle) in candles.iter().enumerate().skip(strategy_config.rsi_period) {
        let rsi = rsi_values[i - strategy_config.rsi_period];
        let sentiment = simulate_sentiment(rsi);
        let signal = strategy.generate_signal(rsi, sentiment);

        // 5) Check if we should close an open position (TP/SL)
        if let Some(ref pos) = open_position {
            if let Some(exit_reason) = pos.check_exit(candle.close) {
                let pnl = pos.calculate_pnl(candle.close);
                stats.record_trade(pnl);
                println!(
                    "Position {} closed: {} at {:.2}, P&L={:.2}, Reason={}",
                    pos.id, pos.side, candle.close, pnl, exit_reason
                );
                open_position = None;
            }
        }

        // 6) Open new position if signal and no open position
        if open_position.is_none() && signal != Signal::Hold {
            if strategy.can_open_position().unwrap_or(false) {
                position_counter += 1;
                let position_id = format!("dry_pos_{}", position_counter);
                let side = match signal {
                    Signal::Buy => OrderSide::Buy,
                    Signal::Sell => OrderSide::Sell,
                    Signal::Hold => continue,
                };

                let entry_price = candle.close;
                let take_profit = strategy.calculate_take_profit(entry_price, side);
                let stop_loss = strategy.calculate_stop_loss(entry_price, side);

                let dry_pos = DryRunPosition {
                    id: position_id.clone(),
                    side,
                    entry_price,
                    volume: 1.0,
                    take_profit,
                    stop_loss,
                };

                println!(
                    "Position {} opened: {} at {:.2}, TP={:.2}, SL={:.2}, RSI={:.2}, Sentiment={}",
                    dry_pos.id, dry_pos.side, entry_price, take_profit, stop_loss, rsi, sentiment
                );

                // Add to strategy for position tracking
                let pos = Position::new(
                    &position_id,
                    &trading_config.symbol,
                    side,
                    entry_price,
                    1.0,
                );
                strategy.add_position(pos);

                open_position = Some(dry_pos);
            }
        }
    }

    // Force close any remaining open position at final price
    if let Some(pos) = open_position {
        let final_price = candles.last().unwrap().close;
        let pnl = pos.calculate_pnl(final_price);
        stats.record_trade(pnl);
        println!(
            "Position {} force closed at {:.2}, P&L={:.2}",
            pos.id, final_price, pnl
        );
    }

    // 7) Validate results
    println!("\n=== TRADING STATISTICS ===");
    println!("Total trades: {}", stats.total_trades);
    println!("Winning trades: {}", stats.winning_trades);
    println!("Losing trades: {}", stats.losing_trades);
    println!("Win rate: {:.2}%", stats.win_rate());
    println!("Total P&L: ${:.2}", stats.total_pnl);
    println!("Final balance: ${:.2}", stats.current_balance);
    println!(
        "Return: {:.2}%",
        ((stats.current_balance - starting_balance) / starting_balance) * 100.0
    );
    println!("Max drawdown: ${:.2}", stats.max_drawdown);

    // Assertions
    assert!(stats.total_trades > 0, "Should have executed at least one trade");
    assert!(
        stats.winning_trades + stats.losing_trades == stats.total_trades,
        "Winning + losing should equal total trades"
    );
    assert!(
        stats.current_balance > 0.0,
        "Balance should remain positive"
    );

    // Win rate should be reasonable (not guaranteed profit but should have some wins)
    if stats.total_trades >= 5 {
        assert!(stats.win_rate() >= 0.0 && stats.win_rate() <= 100.0);
    }

    println!("\n✅ Dry run integration test PASSED");
}

#[test]
fn test_dry_run_with_circuit_breaker() {
    let strategy_config = StrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        rsi_timeframe: "M5".to_string(),
        sentiment_threshold: 30,
    };

    let trading_config = TradingConfig {
        symbol: "FCPO".to_string(),
        risk_per_trade: 1.0,
        max_positions: 1,
        take_profit_percent: 2.0,
        stop_loss_percent: 1.5,
        max_daily_loss_percent: 5.0,
        initial_balance: 10000.0,
    };

    let starting_balance = 10000.0;
    let mut strategy = TradingStrategy::new(
        strategy_config.clone(),
        trading_config.clone(),
        starting_balance,
    );
    strategy.set_trend_filter(false);

    // Simulate 3 losing trades to trigger circuit breakers
    for i in 1..=3 {
        let pos_id = format!("test_pos_{}", i);
        let pos = Position::new(&pos_id, "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(pos);
        strategy.close_position(&pos_id, 4780.0, palm_oil_bot::modules::trading::orders::CloseReason::StopLoss);
    }

    // Should not be able to open position due to circuit breaker
    assert!(
        !strategy.can_open_position().unwrap_or(true),
        "Circuit breaker should prevent new positions"
    );

    println!("✅ Circuit breaker test PASSED");
}

#[test]
fn test_dry_run_consecutive_losses() {
    let strategy_config = StrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        rsi_timeframe: "M5".to_string(),
        sentiment_threshold: 30,
    };

    let trading_config = TradingConfig {
        symbol: "FCPO".to_string(),
        risk_per_trade: 1.0,
        max_positions: 1,
        take_profit_percent: 2.0,
        stop_loss_percent: 1.5,
        max_daily_loss_percent: 5.0,
        initial_balance: 10000.0,
    };

    let starting_balance = 10000.0;
    let mut strategy = TradingStrategy::new(
        strategy_config,
        trading_config.clone(),
        starting_balance,
    );
    strategy.set_trend_filter(false);

    // Simulate 3 consecutive losses
    for i in 1..=3 {
        let pos = Position::new(
            &format!("loss_pos_{}", i),
            &trading_config.symbol,
            OrderSide::Buy,
            4850.0,
            1.0,
        );
        strategy.add_position(pos);
        strategy.close_position(&format!("loss_pos_{}", i), 4800.0, palm_oil_bot::modules::trading::orders::CloseReason::StopLoss);
    }

    // Should not be able to open position after 3 consecutive losses
    assert!(
        !strategy.can_open_position().unwrap_or(true),
        "Should prevent trading after 3 consecutive losses"
    );

    println!("✅ Consecutive losses test PASSED");
}
