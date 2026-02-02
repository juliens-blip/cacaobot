//! Parameter optimization tool for Palm Oil Trading Bot
//!
//! Performs grid search on strategy parameters to find optimal configuration.
//! Tests 192 combinations (4×4×4×3) and ranks by profit factor.

use chrono::{DateTime, Utc};
use palm_oil_bot::modules::trading::{indicators::RsiCalculator, orders::OrderSide};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::cmp::Ordering;

const INITIAL_BALANCE: f64 = 10000.0;
const NUM_CANDLES: usize = 1000;
const START_PRICE: f64 = 4850.0;
const VOLATILITY: f64 = 1.5;
const SEED: u64 = 42;

#[derive(Debug, Clone)]
struct Candle {
    #[allow(dead_code)]
    timestamp: DateTime<Utc>,
    #[allow(dead_code)]
    open: f64,
    #[allow(dead_code)]
    high: f64,
    #[allow(dead_code)]
    low: f64,
    close: f64,
}

#[derive(Debug, Clone)]
struct StrategyParams {
    rsi_oversold: f64,
    rsi_overbought: f64,
    take_profit: f64,
    stop_loss: f64,
}

impl std::fmt::Display for StrategyParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RSI[{:.0},{:.0}] TP[{:.1}%] SL[{:.1}%]",
            self.rsi_oversold, self.rsi_overbought, self.take_profit, self.stop_loss
        )
    }
}

#[derive(Debug, Clone)]
struct BacktestResult {
    params: StrategyParams,
    total_trades: u32,
    #[allow(dead_code)] // Used in debug output and future reporting
    winning_trades: u32,
    #[allow(dead_code)] // Used in debug output and future reporting
    total_pnl: f64,
    pnl_percent: f64,
    profit_factor: f64,
    win_rate: f64,
    max_drawdown_percent: f64,
}

fn generate_price_data(rng: &mut ChaCha8Rng, num_candles: usize, start_price: f64, volatility: f64) -> Vec<Candle> {
    use rand::Rng;
    let mut candles = Vec::with_capacity(num_candles);
    let mut current_price = start_price;
    let mut timestamp = Utc::now() - chrono::Duration::hours(num_candles as i64);

    for _ in 0..num_candles {
        let change = rng.gen_range(-volatility..volatility);
        let new_price = current_price * (1.0 + change / 100.0);

        let high = current_price.max(new_price) * (1.0 + rng.gen_range(0.0..0.3) / 100.0);
        let low = current_price.min(new_price) * (1.0 - rng.gen_range(0.0..0.3) / 100.0);

        candles.push(Candle {
            timestamp,
            open: current_price,
            high,
            low,
            close: new_price,
        });

        current_price = new_price;
        timestamp += chrono::Duration::hours(1);
    }

    candles
}

fn simulate_sentiment(rng: &mut ChaCha8Rng, rsi: f64) -> i32 {
    use rand::Rng;

    let base_sentiment = if rsi < 30.0 {
        rng.gen_range(20..60)
    } else if rsi > 70.0 {
        rng.gen_range(-60..-20)
    } else {
        rng.gen_range(-40..40)
    };

    let noise = rng.gen_range(-20..20);
    (base_sentiment + noise).clamp(-100, 100)
}

fn run_backtest(candles: &[Candle], params: &StrategyParams, seed: u64) -> BacktestResult {
    let mut rng = ChaCha8Rng::seed_from_u64(seed + 1000);
    let mut rsi_calc = RsiCalculator::new(14);
    let mut balance = INITIAL_BALANCE;
    let mut peak_balance = INITIAL_BALANCE;
    let mut max_drawdown = 0.0;

    let mut total_trades = 0u32;
    let mut winning_trades = 0u32;
    let mut total_wins = 0.0;
    let mut total_losses = 0.0;

    let mut current_position: Option<(OrderSide, f64, f64)> = None;

    for candle in candles.iter() {
        let price = candle.close;
        let rsi_opt = rsi_calc.add_price(price);

        if let Some(rsi) = rsi_opt {
            let sentiment = simulate_sentiment(&mut rng, rsi);

            if let Some((side, entry_price, volume)) = current_position.take() {
                let pnl = match side {
                    OrderSide::Buy => (price - entry_price) * volume,
                    OrderSide::Sell => (entry_price - price) * volume,
                };

                let pnl_percent = (pnl / entry_price) * 100.0;

                let should_close = pnl_percent >= params.take_profit
                    || pnl_percent <= -params.stop_loss;

                if should_close {
                    balance += pnl;
                    total_trades += 1;

                    if pnl > 0.0 {
                        winning_trades += 1;
                        total_wins += pnl;
                    } else {
                        total_losses += pnl.abs();
                    }

                    if balance > peak_balance {
                        peak_balance = balance;
                    }

                    let drawdown = peak_balance - balance;
                    if drawdown > max_drawdown {
                        max_drawdown = drawdown;
                    }
                } else {
                    current_position = Some((side, entry_price, volume));
                }
            }

            if current_position.is_none() {
                let should_buy = rsi < params.rsi_oversold && sentiment > 30;
                let should_sell = rsi > params.rsi_overbought && sentiment < -30;

                if should_buy {
                    current_position = Some((OrderSide::Buy, price, 1.0));
                } else if should_sell {
                    current_position = Some((OrderSide::Sell, price, 1.0));
                }
            }
        }
    }

    if let Some((side, entry_price, volume)) = current_position {
        let final_price = candles.last().unwrap().close;
        let final_pnl = match side {
            OrderSide::Buy => (final_price - entry_price) * volume,
            OrderSide::Sell => (entry_price - final_price) * volume,
        };

        balance += final_pnl;
        total_trades += 1;

        if final_pnl > 0.0 {
            winning_trades += 1;
            total_wins += final_pnl;
        } else {
            total_losses += final_pnl.abs();
        }
    }

    let total_pnl = balance - INITIAL_BALANCE;
    let pnl_percent = (total_pnl / INITIAL_BALANCE) * 100.0;
    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let avg_win = if winning_trades > 0 {
        total_wins / winning_trades as f64
    } else {
        0.0
    };
    let avg_loss = if total_trades > winning_trades {
        total_losses / (total_trades - winning_trades) as f64
    } else {
        0.0
    };

    let profit_factor = if avg_loss > 0.0 {
        avg_win / avg_loss
    } else if avg_win > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let max_drawdown_percent = (max_drawdown / INITIAL_BALANCE) * 100.0;

    BacktestResult {
        params: params.clone(),
        total_trades,
        winning_trades,
        total_pnl,
        pnl_percent,
        profit_factor,
        win_rate,
        max_drawdown_percent,
    }
}

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║     🌴 PALM OIL BOT - PARAMETER OPTIMIZATION 🌴                  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let rsi_oversold_values = [20.0, 25.0, 30.0, 35.0];
    let rsi_overbought_values = [65.0, 70.0, 75.0, 80.0];
    let take_profit_values = [1.5, 2.0, 2.5, 3.0];
    let stop_loss_values = [1.0, 1.5, 2.0];

    let total_combinations = rsi_oversold_values.len()
        * rsi_overbought_values.len()
        * take_profit_values.len()
        * stop_loss_values.len();

    println!("║ Grid Search Configuration:                                       ║");
    println!("║   RSI Oversold   : {:?}                       ║", rsi_oversold_values);
    println!("║   RSI Overbought : {:?}                       ║", rsi_overbought_values);
    println!("║   Take Profit    : {:?}                          ║", take_profit_values);
    println!("║   Stop Loss      : {:?}                               ║", stop_loss_values);
    println!("║   Total runs     : {}                                          ║", total_combinations);
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║ Generating synthetic price data ({} candles)...                ║", NUM_CANDLES);

    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    let candles = generate_price_data(&mut rng, NUM_CANDLES, START_PRICE, VOLATILITY);

    println!("║ Running {} backtests...                                        ║", total_combinations);
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut results: Vec<BacktestResult> = Vec::with_capacity(total_combinations);
    let mut count = 0;

    for &rsi_oversold in &rsi_oversold_values {
        for &rsi_overbought in &rsi_overbought_values {
            for &take_profit in &take_profit_values {
                for &stop_loss in &stop_loss_values {
                    let params = StrategyParams {
                        rsi_oversold,
                        rsi_overbought,
                        take_profit,
                        stop_loss,
                    };

                    let result = run_backtest(&candles, &params, SEED);
                    results.push(result);

                    count += 1;
                    if count % 48 == 0 {
                        print!("\rProgress: {}/{} ({:.0}%)", count, total_combinations, (count as f64 / total_combinations as f64) * 100.0);
                    }
                }
            }
        }
    }
    println!("\rProgress: {}/{} (100%)      ", count, total_combinations);

    results.sort_by(|a, b| {
        b.profit_factor.partial_cmp(&a.profit_factor).unwrap_or(Ordering::Equal)
    });

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    OPTIMIZATION RESULTS                          ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║ Rank │ Parameters              │ PF     │ P&L     │ WR    │ Trades║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    for (i, result) in results.iter().take(10).enumerate() {
        let rank = i + 1;
        let pf_str = if result.profit_factor.is_infinite() {
            "  INF".to_string()
        } else {
            format!("{:5.2}", result.profit_factor)
        };

        let pnl_sign = if result.pnl_percent >= 0.0 { "+" } else { "" };

        println!(
            "║  {:2}  │ {:22} │ {} │ {:}{:5.1}% │ {:4.1}% │  {:3}  ║",
            rank,
            result.params,
            pf_str,
            pnl_sign,
            result.pnl_percent,
            result.win_rate,
            result.total_trades,
        );
    }

    println!("╚══════════════════════════════════════════════════════════════════╝");

    let above_target: Vec<_> = results.iter().filter(|r| r.profit_factor >= 1.5 && r.profit_factor.is_finite()).collect();

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                         SUMMARY                                  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║ Combinations with PF >= 1.5: {:<4}                                ║", above_target.len());

    if let Some(best) = results.first() {
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ BEST PARAMETERS:                                                 ║");
        println!("║   RSI Oversold   : {:<5.0}                                        ║", best.params.rsi_oversold);
        println!("║   RSI Overbought : {:<5.0}                                        ║", best.params.rsi_overbought);
        println!("║   Take Profit    : {:<5.1}%                                       ║", best.params.take_profit);
        println!("║   Stop Loss      : {:<5.1}%                                       ║", best.params.stop_loss);
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║ BEST PERFORMANCE:                                                ║");
        let pf_str = if best.profit_factor.is_infinite() {
            "INF".to_string()
        } else {
            format!("{:.2}", best.profit_factor)
        };
        let pnl_sign = if best.pnl_percent >= 0.0 { "+" } else { "" };
        println!("║   Profit Factor  : {:<10}                                    ║", pf_str);
        println!("║   Total P&L      : {:}{:.2}%                                       ║", pnl_sign, best.pnl_percent);
        println!("║   Win Rate       : {:.1}%                                         ║", best.win_rate);
        println!("║   Max Drawdown   : {:.2}%                                        ║", best.max_drawdown_percent);
        println!("║   Total Trades   : {}                                            ║", best.total_trades);
    }

    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    if above_target.len() >= 5 {
        println!("✅ Found {} parameter combinations exceeding target PF of 1.5!", above_target.len());
    } else if !above_target.is_empty() {
        println!("⚠️  Found {} combinations with PF >= 1.5. Consider expanding search.", above_target.len());
    } else {
        println!("❌ No combinations reached target PF of 1.5. Strategy needs refinement.");
    }
}
