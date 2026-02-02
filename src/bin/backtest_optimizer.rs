//! Backtest optimizer with grid search.
//!
//! Usage: cargo run --bin backtest-optimizer

use chrono::{DateTime, Utc};
use palm_oil_bot::modules::trading::{
    indicators::RsiCalculator,
    orders::{OrderSide, Position},
    strategy::{Signal, TradingStrategy},
};
use palm_oil_bot::config::{StrategyConfig, TradingConfig};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::fs::File;
use std::io::{BufWriter, Write};

const INITIAL_BALANCE: f64 = 10000.0;
const START_PRICE: f64 = 4850.0;
const NUM_CANDLES: usize = 1500;
const VOLATILITY_PERCENT: f64 = 1.5;
const SENTIMENT_THRESHOLD: i32 = 30;
const CSV_OUTPUT: &str = "backtest_results.csv";

#[derive(Debug, Clone)]
struct Candle {
    timestamp: DateTime<Utc>,
    close: f64,
}

#[derive(Debug, Clone, Copy)]
struct Params {
    rsi_buy: f64,
    rsi_sell: f64,
    tp: f64,
    sl: f64,
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    profit_factor: f64,
    win_rate: f64,
}

fn main() -> anyhow::Result<()> {
    let candles = generate_price_data(NUM_CANDLES, START_PRICE, VOLATILITY_PERCENT);

    let mut writer = BufWriter::new(File::create(CSV_OUTPUT)?);
    writeln!(
        writer,
        "rsi_buy,rsi_sell,tp,sl,profit_factor,win_rate"
    )?;

    let mut best: Option<(Params, Metrics)> = None;

    for rsi_buy in (20..=35).step_by(5) {
        for rsi_sell in (65..=80).step_by(5) {
            for tp in [1.5, 2.0, 2.5, 3.0] {
                for sl in [1.0, 1.5, 2.0] {
                    let params = Params {
                        rsi_buy: rsi_buy as f64,
                        rsi_sell: rsi_sell as f64,
                        tp,
                        sl,
                    };
                    let metrics = run_simulation(&candles, params);

                    writeln!(
                        writer,
                        "{:.1},{:.1},{:.1},{:.1},{:.4},{:.2}",
                        params.rsi_buy,
                        params.rsi_sell,
                        params.tp,
                        params.sl,
                        metrics.profit_factor,
                        metrics.win_rate
                    )?;

                    if metrics.profit_factor > 1.5 {
                        best = match best {
                            Some((best_params, best_metrics)) => {
                                if metrics.profit_factor > best_metrics.profit_factor {
                                    Some((params, metrics))
                                } else {
                                    Some((best_params, best_metrics))
                                }
                            }
                            None => Some((params, metrics)),
                        };
                    }
                }
            }
        }
    }

    writer.flush()?;

    println!("CSV results written to {}", CSV_OUTPUT);
    match best {
        Some((params, metrics)) => {
            println!("Best profit factor > 1.5 found:");
            println!(
                "  rsi_buy={:.1}, rsi_sell={:.1}, tp={:.1}%, sl={:.1}%",
                params.rsi_buy, params.rsi_sell, params.tp, params.sl
            );
            println!(
                "  profit_factor={:.4}, win_rate={:.2}%",
                metrics.profit_factor, metrics.win_rate
            );
        }
        None => {
            println!("No parameter set produced profit_factor > 1.5");
        }
    }

    Ok(())
}

fn generate_price_data(num_candles: usize, start_price: f64, volatility: f64) -> Vec<Candle> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut candles = Vec::with_capacity(num_candles);
    let mut current_price = start_price;
    let mut timestamp = Utc::now() - chrono::Duration::hours(num_candles as i64);

    for _ in 0..num_candles {
        let change = rng.gen_range(-volatility..volatility);
        let new_price = current_price * (1.0 + change / 100.0);

        candles.push(Candle {
            timestamp,
            close: new_price,
        });

        current_price = new_price;
        timestamp += chrono::Duration::hours(1);
    }

    candles
}

fn run_simulation(candles: &[Candle], params: Params) -> Metrics {
    let trading_config = TradingConfig {
        symbol: "FCPO".to_string(),
        risk_per_trade: 0.01,
        take_profit_percent: params.tp,
        stop_loss_percent: params.sl,
        max_positions: 1,
        max_daily_loss_percent: 5.0,
        initial_balance: 10000.0,
    };

    let strategy_config = StrategyConfig {
        rsi_period: 14,
        rsi_oversold: params.rsi_buy,
        rsi_overbought: params.rsi_sell,
        rsi_timeframe: "5m".to_string(),
        sentiment_threshold: SENTIMENT_THRESHOLD,
    };

    let mut strategy = TradingStrategy::new(strategy_config, trading_config, INITIAL_BALANCE);
    let mut rsi_calc = RsiCalculator::new(14);

    let mut position: Option<Position> = None;
    let mut total_trades = 0u32;
    let mut winning_trades = 0u32;
    let mut total_wins = 0.0;
    let mut total_losses = 0.0;

    for candle in candles {
        let price = candle.close;
        strategy.update_price(price);

        let rsi_opt = rsi_calc.add_price(price);
        if rsi_opt.is_none() {
            continue;
        }
        let rsi = rsi_opt.unwrap();

        if let Some(mut open_position) = position.take() {
            open_position.update_price(price);
            let tp_hit = strategy.check_take_profit(&open_position, price);
            let sl_hit = strategy.check_stop_loss(&open_position, price);

            if tp_hit || sl_hit {
                let pnl = open_position.calculate_pnl(price);
                total_trades += 1;
                if pnl > 0.0 {
                    winning_trades += 1;
                    total_wins += pnl;
                } else {
                    total_losses += pnl.abs();
                }
            } else {
                position = Some(open_position);
            }
        }

        if position.is_none() {
            let sentiment = simulate_sentiment(rsi, params.rsi_buy, params.rsi_sell);
            let signal = strategy.generate_signal(rsi, sentiment);

            if signal != Signal::Hold {
                let side = match signal {
                    Signal::Buy => OrderSide::Buy,
                    Signal::Sell => OrderSide::Sell,
                    Signal::Hold => continue,
                };

                position = Some(Position::new(
                    format!("opt_{}", candle.timestamp.timestamp()),
                    "FCPO",
                    side,
                    price,
                    1.0,
                ));
            }
        }
    }

    if let Some(open_position) = position {
        let final_price = candles.last().map(|c| c.close).unwrap_or(open_position.entry_price);
        let pnl = open_position.calculate_pnl(final_price);
        total_trades += 1;
        if pnl > 0.0 {
            winning_trades += 1;
            total_wins += pnl;
        } else {
            total_losses += pnl.abs();
        }
    }

    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if total_losses > 0.0 {
        total_wins / total_losses
    } else if total_wins > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    Metrics {
        profit_factor,
        win_rate,
    }
}

fn simulate_sentiment(rsi: f64, rsi_buy: f64, rsi_sell: f64) -> i32 {
    if rsi <= rsi_buy {
        SENTIMENT_THRESHOLD + 20
    } else if rsi >= rsi_sell {
        -(SENTIMENT_THRESHOLD + 20)
    } else {
        0
    }
}
